//! The answer probe: records the RAW answers of write calls whose answers the
//! crate does not yet type or anchor. Writes PERMANENT documents to the sevDesk
//! TEST account (the probe aborts on any other account before the first
//! write), so it is its own test target (`test = false` in Cargo.toml) and runs
//! only on request:
//!
//! `just test-live --test answer_probe case1`
//!
//! Sequence (every request body is a serialized crate request type; only the
//! transport is the harness's raw call, so the answer can be recorded):
//!
//! 1. invoice draft (`SaveInvoice`) -> `PUT sendBy` (`SendBy::FINALIZE`) ->
//!    `PUT bookAmount` (`BookDocumentAmount`, FULL_PAYMENT) -> `PUT resetToOpen`
//!    -> `PUT changeParameter` (`DocumentParameter`) -> `POST cancelInvoice`
//! 2. credit note draft (`SaveCreditNote`, `refSrcInvoice` = an EXISTING
//!    enshrined invoice found read-only) -> `PUT sendBy` -> `PUT bookAmount` ->
//!    `PUT resetToOpen` -> `PUT changeParameter`
//! 3. voucher draft -> released (status 100) -> `PUT resetToDraft`
//!
//! Budget: 4 permanent documents (invoice, cancellation invoice, credit note,
//! voucher: the voucher is deleted again) and 3 consumed numbers. The two
//! booking transactions are deleted again after `resetToOpen` freed them.
//! Inputs (customer, payment method, check account, origin invoice) are found
//! read-only. Output per call: the raw scrubbed body in
//! `target/tmp/captures/answer_probe_<call>.json`, and one stderr line
//! `ANSWER\t<call>\tHTTP <status>\t<body head>`; `CREATED <Object> <id>` and
//! `NUMBER <Object> <id> <number>` lines count documents and numbers.
//! A failed call is recorded, the document re-read (`..._reread`), and the
//! next independent step runs; no write is retried.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness files are shared with live_write and use only some of the helpers; a failure in a test helper is a test failure"
)]

#[path = "../live_write/docs.rs"]
mod docs;
#[path = "../live_write/harness.rs"]
mod harness;
#[path = "../common/scrub.rs"]
mod scrub;

use docs::*;
use harness::*;
use reqwest::Method;
use sevdesk::*;
use time::OffsetDateTime;
use time::macros::{date, datetime};

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;
/// The name prefix of the contacts the live tests create.
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";
/// Net price of the one position of the invoice, in cents.
const NET: i64 = 1000;
/// Net price of the one position of the credit note, in cents.
const CREDIT_NET: i64 = 500;

/// Aborts unless `GET /SevClient` names the test account.
async fn verify_test_account(ctx: &Ctx) {
    let (status, text) = ctx.raw(Method::GET, "/SevClient", None, None).await;
    assert!(status < 300, "GET /SevClient answered {status}");
    let id = raw_one(&text)["id"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok());
    assert_eq!(
        id,
        Some(TEST_SEV_CLIENT),
        "not the test account: aborting before the write"
    );
}

/// An existing test contact, found by name (read-only) and verified by GET.
async fn existing_customer(ctx: &Ctx) -> Customer {
    let found = ctx.c.list_contacts_by_name(CONTACT_PREFIX).await.unwrap();
    let contact = found
        .into_iter()
        .find(|c| {
            c.name
                .as_deref()
                .is_some_and(|n| n.starts_with(CONTACT_PREFIX))
        })
        .unwrap_or_else(|| panic!("no existing contact named `{CONTACT_PREFIX}…`: aborting"));
    customer_of(ctx, contact.id).await
}

async fn customer_of(ctx: &Ctx, id: ContactId) -> Customer {
    let verified = ctx.c.contact(id).await.unwrap();
    assert_eq!(verified.id, id);
    let name = verified.name.clone().expect("the contact has a name");
    Customer {
        contact: verified,
        name,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

/// An EXISTING enshrined invoice of the test account (read-only list) that no
/// cancellation invoice cancels and that is big enough for the credit note.
/// One of the probe customer's is preferred.
async fn existing_origin(ctx: &Ctx, customer: &Customer) -> Invoice {
    let query = InvoiceQuery {
        page: Some(Page::new(1000, 0).unwrap()),
        ..Default::default()
    };
    let all = ctx.c.list_invoices(&query).await.unwrap();
    let mut usable: Vec<&Invoice> = all
        .iter()
        .filter(|i| {
            i.invoice_type == Some(InvoiceType::Invoice)
                && i.enshrined.is_some()
                && matches!(i.status, Some(InvoiceStatus::Open | InvoiceStatus::Paid))
                && i.contact.is_some()
                && i.sum_gross
                    .is_some_and(|g| g.minor() >= CREDIT_NET * 119 / 100)
                && !all.iter().any(|other| other.cancels(i.id))
        })
        .collect();
    usable.sort_by_key(|i| i.contact.map(|c| c.id) != Some(customer.contact.id));
    let origin = (*usable.first().unwrap_or_else(|| {
        panic!(
            "no existing enshrined, uncancelled invoice among {}: aborting before the write",
            all.len()
        )
    }))
    .clone();
    eprintln!(
        "SETUP origin invoice {} (own customer: {})",
        origin.id,
        origin.contact.map(|c| c.id) == Some(customer.contact.id)
    );
    origin
}

/// One recorded write: the scrubbed answer goes to
/// `answer_probe_<name>.json`, status and body head to stderr. A failed call
/// (status 300 or more, or an `error` key in the body) is re-read and not
/// retried. Answers `true` when the call looks successful.
async fn record(
    ctx: &Ctx,
    name: &str,
    method: Method,
    path: &str,
    body: Option<String>,
    reread: &str,
) -> (bool, String) {
    let (status, text) = ctx
        .raw(method, path, body, Some(&format!("answer_probe_{name}")))
        .await;
    let head: String = scrub::scrub_capture(&text).chars().take(300).collect();
    eprintln!("ANSWER\t{name}\tHTTP {status}\t{}", head.replace('\n', " "));
    let failed = status >= 300 || text.contains("\"error\"");
    if failed {
        eprintln!("ANSWER-FAILED\t{name}\tre-reading {reread}");
        let (s, t) = ctx
            .raw(
                Method::GET,
                reread,
                None,
                Some(&format!("answer_probe_{name}_reread")),
            )
            .await;
        let head: String = scrub::scrub_capture(&t).chars().take(300).collect();
        eprintln!(
            "ANSWER\t{name}_reread\tHTTP {s}\t{}",
            head.replace('\n', " ")
        );
    }
    (!failed, text)
}

/// `GET /<obj>/<id>`, recorded under `name`; `None` when it did not answer 2xx.
async fn read(ctx: &Ctx, obj: &str, id: i64, name: &str) -> Option<serde_json::Value> {
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}/{id}"),
            None,
            Some(&format!("answer_probe_{name}")),
        )
        .await;
    (status < 300).then(|| raw_one(&text))
}

fn text_of(v: &serde_json::Value, key: &str) -> String {
    match &v[key] {
        serde_json::Value::Null => "-".to_owned(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Prints the `NUMBER` line of a finalized document.
fn print_number(obj: &str, id: i64, doc: &serde_json::Value) {
    let key = if obj == "Invoice" {
        "invoiceNumber"
    } else {
        "creditNoteNumber"
    };
    eprintln!(
        "NUMBER {obj} {id} {}\tstatus {}\tenshrined {}",
        text_of(doc, key),
        text_of(doc, "status"),
        text_of(doc, "enshrined")
    );
}

fn booking(signed: i64, tx: TransactionId) -> BookDocumentAmount {
    BookDocumentAmount {
        amount: eur(signed),
        date: OffsetDateTime::now_utc(),
        booking_type: BookingType::FullPayment,
        check_account: ObjectRef::new(account(ZEIT)),
        check_account_transaction: ObjectRef::new(tx),
    }
}

/// The write sequence of one invoice or credit note: finalize, book, reset,
/// parameter. Stops after a failed finalize that left the draft a draft.
/// Returns the booking transaction (to delete it afterwards).
async fn document_chain(
    ctx: &Ctx,
    obj: &str,
    id: i64,
    signed_gross: i64,
    prefix: &str,
) -> Option<TransactionId> {
    let path = format!("/{obj}/{id}");
    let (sent, _) = record(
        ctx,
        &format!("{prefix}_sendBy"),
        Method::PUT,
        &format!("{path}/sendBy"),
        Some(json(&SendBy::FINALIZE)),
        &path,
    )
    .await;
    let after_send = read(ctx, obj, id, &format!("{prefix}_after_sendBy")).await;
    if let Some(doc) = &after_send {
        print_number(obj, id, doc);
    }
    if !sent && after_send.is_none_or(|d| text_of(&d, "status") == "100") {
        eprintln!("ANSWER-NOTE\t{prefix}: sendBy left a draft, chain stops");
        return None;
    }

    let at = datetime!(2026-11-05 12:00 +1);
    let tx = match ctx
        .c
        .create_transaction(&ctx.new_transaction(ZEIT, signed_gross, at))
        .await
    {
        Ok(t) => t.id,
        Err(e) => {
            eprintln!("ANSWER-NOTE\t{prefix}: creating the transaction failed: {e}");
            return None;
        }
    };
    eprintln!("CREATED CheckAccountTransaction {} ({prefix})", tx.get());
    record(
        ctx,
        &format!("{prefix}_bookAmount"),
        Method::PUT,
        &format!("{path}/bookAmount"),
        Some(json(&booking(signed_gross, tx))),
        &path,
    )
    .await;
    record(
        ctx,
        &format!("{prefix}_resetToOpen"),
        Method::PUT,
        &format!("{path}/resetToOpen"),
        None,
        &path,
    )
    .await;
    record(
        ctx,
        &format!("{prefix}_changeParameter"),
        Method::PUT,
        &format!("{path}/changeParameter"),
        Some(json(&DocumentParameter::Language(DocumentLanguage::EnUs))),
        &path,
    )
    .await;
    Some(tx)
}

/// Best effort: a transaction is deletable only once nothing is booked on it.
async fn delete_transaction(ctx: &Ctx, tx: Option<TransactionId>) {
    if let Some(tx) = tx
        && let Err(e) = ctx.c.delete_transaction(tx).await
    {
        eprintln!("ANSWER-NOTE\tdeleting transaction {} failed: {e}", tx.get());
    }
}

async fn invoice_part(ctx: &Ctx, customer: &Customer, person: SevUserId, pm: PaymentMethodId) {
    let note = format!("{}-invoice", ctx.marker);
    let spec = InvoiceSpec {
        customer,
        person,
        note,
        b2b: true,
        payment_method: Some(pm),
        with_address: true,
    };
    let saved = ctx
        .c
        .save_invoice(&SaveInvoice::new(
            invoice_draft(&spec),
            vec![net_position(1, NET, "SDRS-ANS Position", None)],
        ))
        .await
        .unwrap();
    let id = saved.invoice.id.get();
    let gross = saved
        .invoice
        .sum_gross
        .map_or(NET * 119 / 100, Amount::minor);
    eprintln!("CREATED Invoice {id}");

    let tx = document_chain(ctx, "Invoice", id, gross, "invoice").await;
    record(
        ctx,
        "invoice_cancelInvoice",
        Method::POST,
        &format!("/Invoice/{id}/cancelInvoice"),
        None,
        &format!("/Invoice/{id}"),
    )
    .await;
    // The cancellation invoice is found by its own answer when it has one.
    let (_, text) = ctx
        .raw(
            Method::GET,
            &format!("/Invoice?customerInternalNote={}-invoice", ctx.marker),
            None,
            Some("answer_probe_invoice_after_cancel_list"),
        )
        .await;
    let listed: Vec<serde_json::Value> =
        serde_json::from_value(raw_objects(&text)).unwrap_or_default();
    for inv in listed {
        let other: Option<i64> = inv["id"].as_str().and_then(|s| s.parse().ok());
        if other.is_some_and(|o| o != id) {
            eprintln!("CREATED Invoice {} (cancellation)", other.unwrap());
            print_number("Invoice", other.unwrap(), &inv);
        }
    }
    read(ctx, "Invoice", id, "invoice_final").await;
    delete_transaction(ctx, tx).await;
}

async fn credit_note_part(ctx: &Ctx, origin: &Invoice, person: SevUserId, pm: PaymentMethodId) {
    let c = customer_of(
        ctx,
        origin
            .contact
            .expect("the origin invoice names a contact")
            .id,
    )
    .await;
    let note = format!("{}-credit-note", ctx.marker);
    let number = ctx.c.next_credit_note_number().await.unwrap();
    let draft = {
        let mut draft = CreditNoteDraft {
            credit_note_number: number,
            contact: ObjectRef::new(c.contact.id),
            contact_person: ObjectRef::new(person),
            credit_note_date: date!(2026 - 10 - 01),
            delivery_date: datetime!(2026-10-01 0:00 +2),
            header: note.clone(),
            tax_rule: ObjectRef::new(TaxRuleId::new(1)),
            tax_text: "Umsatzsteuer 19%".into(),
            booking_category: CreditNoteCategory::Underachievement,
            currency: CurrencyCode::EUR,
            show_net: true,
            customer_internal_note: note,
            options: Default::default(),
        };
        draft.options.delivery_date_until = Some(datetime!(2026-10-31 0:00 +1));
        draft.options.ref_src_invoice = Some(ObjectRef::new(origin.id));
        draft.options.address_name = Some(c.name.clone());
        draft.options.address_street = Some(c.street.into());
        draft.options.address_zip = Some(c.zip.into());
        draft.options.address_city = Some(c.city.into());
        draft.options.address_country = Some(ObjectRef::new(GERMANY));
        draft.options.payment_method = Some(ObjectRef::new(pm));
        draft.options.is_e_invoice = true;
        draft
    };
    let saved = ctx
        .c
        .save_credit_note(&SaveCreditNote::new(
            draft,
            vec![net_position(1, CREDIT_NET, "SDRS-ANS Position", None)],
        ))
        .await
        .unwrap();
    let id = saved.credit_note.id.get();
    let gross = saved
        .credit_note
        .sum_gross
        .map_or(CREDIT_NET * 119 / 100, Amount::minor);
    eprintln!("CREATED CreditNote {id}");
    // A credit note books the negative amount.
    let tx = document_chain(ctx, "CreditNote", id, -gross, "credit_note").await;
    read(ctx, "CreditNote", id, "credit_note_final").await;
    delete_transaction(ctx, tx).await;
}

async fn voucher_part(ctx: &Ctx) {
    let saved = ctx
        .c
        .save_voucher(&SaveVoucher {
            voucher: ctx.draft(),
            positions: vec![position(35, "fee")],
            filename: None,
        })
        .await
        .unwrap();
    let id = saved.voucher.id;
    eprintln!("CREATED Voucher {}", id.get());
    // Release through the API, as live_write does (status 100).
    let mut released = ctx.draft();
    released.options.id = Some(id);
    released.status = VoucherStatus::Open;
    let released = ctx
        .c
        .save_voucher(&SaveVoucher {
            voucher: released,
            positions: with_ids(&saved, &[(35, "fee")]),
            filename: None,
        })
        .await
        .unwrap();
    assert_eq!(released.voucher.status, Some(VoucherStatus::Open));
    record(
        ctx,
        "voucher_resetToDraft",
        Method::PUT,
        &format!("/Voucher/{}/resetToDraft", id.get()),
        None,
        &format!("/Voucher/{}", id.get()),
    )
    .await;
    read(ctx, "Voucher", id.get(), "voucher_final").await;
    if let Err(e) = ctx.c.delete_voucher(id).await {
        eprintln!("ANSWER-NOTE\tdeleting voucher {} failed: {e}", id.get());
    }
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_record_write_answers() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the answer probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-ANS-{ts}"));
    verify_test_account(&ctx).await;

    // Read-only inputs.
    let methods = ctx.c.list_payment_methods().await.unwrap();
    let pm = methods
        .iter()
        .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
        .expect("a payment method with electronicInvoiceId 68")
        .id;
    let check = ctx.c.check_account(account(ZEIT)).await.unwrap();
    assert_eq!(check.id, account(ZEIT));
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    let origin = existing_origin(&ctx, &customer).await;
    eprintln!(
        "SETUP contact {} payment method {} check account {}",
        customer.contact.id.get(),
        pm.get(),
        check.id.get()
    );

    invoice_part(&ctx, &customer, person, pm).await;
    credit_note_part(&ctx, &origin, person, pm).await;
    voucher_part(&ctx).await;
}
