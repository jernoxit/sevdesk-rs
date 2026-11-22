//! The credit rounding probe: measures where sevDesk rounds the tax of a net
//! document with two positions of EUR 0.03 net at 19 % (0.0057 per position:
//! 0.01 per line, 0.01 on the sum of 0.06 net is 0.0114 -> 0.01, per line
//! rounded twice it is 0.02), and whether a full credit note of that invoice
//! comes out with the same gross, as a draft and as a finalized document.
//! Writes 2 PERMANENT documents to the sevDesk TEST account (the probe aborts
//! on any other account before the first write), so it is its own test target
//! (`test = false` in Cargo.toml) and runs only on request:
//!
//! `just test-live --test credit_rounding_probe case1`
//!
//! Sequence (all writes through the crate's public API, every read raw):
//! 1. `GET /SevClient` (account check), then read-only inputs: contact,
//!    contact person, payment method.
//! 2. `save_invoice` (net, two positions) -> `send_invoice_by(FINALIZE)`
//!    (consumes ONE RE number) -> raw `GET /Invoice/{id}` and
//!    `GET /InvoicePos?invoice[id]=..&invoice[objectName]=Invoice`.
//! 3. `next_credit_note_number` (read-only) -> `save_credit_note` (full credit
//!    of that invoice, same positions) -> raw read of head and positions.
//! 4. `send_credit_note_by(FINALIZE)` (consumes ONE GU number) -> raw read.
//!
//! Budget: 2 permanent documents, at most 1 RE and 1 GU number. No write is
//! retried; after a failed write the answer may still have been written, so
//! the run re-reads by the run marker and stops.
//! Output: scrubbed raw answers in `target/tmp/captures/credit_rounding_*.json`
//! and the stderr lines `CREATED`, `NUMBER`, `ROUND` and `ROUND-VERDICT`.
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
use serde_json::Value;
use sevdesk::*;
use time::OffsetDateTime;
use time::macros::{date, datetime};

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;
/// The name prefix of the contacts the live tests create.
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";
/// Net price of each of the two positions, in cents.
const NET: i64 = 3;

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
    let verified = ctx.c.contact(contact.id).await.unwrap();
    assert_eq!(verified.id, contact.id);
    let name = verified.name.clone().expect("the contact has a name");
    Customer {
        contact: verified,
        name,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

fn text_of(v: &Value, key: &str) -> String {
    match &v[key] {
        Value::Null => "-".to_owned(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The two net positions: quantity 1, EUR 0.03 net, 19 %.
fn positions() -> Vec<DocumentPositionDraft> {
    vec![
        net_position(1, NET, "SDRS-RND Position A", None),
        net_position(1, NET, "SDRS-RND Position B", None),
    ]
}

/// The sums of one document read back raw.
struct Sums {
    id: String,
    number: String,
    status: String,
    enshrined: String,
    net: String,
    tax: String,
    gross: String,
    /// `net/tax/gross` per position.
    positions: Vec<[String; 3]>,
}

impl Sums {
    fn positions_text(&self) -> String {
        self.positions
            .iter()
            .map(|p| p.join("/"))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// Reads head and positions of a document raw and records both under
/// `credit_rounding_<name>_head` / `_pos`. `None` when a read failed.
async fn read_sums(ctx: &Ctx, obj: &str, id: i64, name: &str) -> Option<Sums> {
    let number_key = if obj == "Invoice" {
        "invoiceNumber"
    } else {
        "creditNoteNumber"
    };
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}/{id}"),
            None,
            Some(&format!("credit_rounding_{name}_head")),
        )
        .await;
    if status >= 300 {
        eprintln!("ROUND-FAILED\tGET /{obj}/{id} answered {status}");
        return None;
    }
    let head = raw_one(&text);
    let ref_key = if obj == "Invoice" {
        "invoice"
    } else {
        "creditNote"
    };
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}Pos?{ref_key}[id]={id}&{ref_key}[objectName]={obj}"),
            None,
            Some(&format!("credit_rounding_{name}_pos")),
        )
        .await;
    if status >= 300 {
        eprintln!("ROUND-FAILED\tGET /{obj}Pos answered {status}");
        return None;
    }
    let list: Vec<Value> = serde_json::from_value(raw_objects(&text)).unwrap_or_default();
    Some(Sums {
        id: id.to_string(),
        number: text_of(&head, number_key),
        status: text_of(&head, "status"),
        enshrined: text_of(&head, "enshrined"),
        net: text_of(&head, "sumNet"),
        tax: text_of(&head, "sumTax"),
        gross: text_of(&head, "sumGross"),
        positions: list
            .iter()
            .map(|p| {
                [
                    text_of(p, "sumNet"),
                    text_of(p, "sumTax"),
                    text_of(p, "sumGross"),
                ]
            })
            .collect(),
    })
}

fn print_round(doc: &str, state: &str, s: &Sums) {
    eprintln!(
        "ROUND\t{doc}\t{state}\t{}\t{}\t{}\t{}\t{}\t{}",
        s.id,
        s.number,
        s.net,
        s.tax,
        s.gross,
        s.positions_text()
    );
    eprintln!(
        "NUMBER {doc} {} {}\tstatus {}\tenshrined {}",
        s.id, s.number, s.status, s.enshrined
    );
}

/// Where the tax of a document was rounded: per line (every position carries
/// 0.02 in total) or on the sum.
fn tax_where(s: &Sums) -> String {
    let per_line_sum: Option<f64> = s.positions.iter().map(|p| p[1].parse::<f64>().ok()).sum();
    format!(
        "head tax {} / positions' tax {:?} (sum {:?}): 0.02 = per line, 0.01 = on sum",
        s.tax,
        s.positions
            .iter()
            .map(|p| p[1].as_str())
            .collect::<Vec<_>>(),
        per_line_sum
    )
}

/// After a failed write: finds what may have been written by the run marker
/// and stops. Nothing is retried.
async fn reread_and_stop(ctx: &Ctx, obj: &str, what: &str, err: &dyn std::fmt::Display) -> ! {
    eprintln!("ROUND-FAILED\t{what} failed: {err}");
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}?customerInternalNote={}", ctx.marker),
            None,
            Some(&format!("credit_rounding_{what}_reread")),
        )
        .await;
    let head: String = scrub::scrub_capture(&text).chars().take(400).collect();
    eprintln!(
        "ROUND-FAILED\tre-read HTTP {status}\t{}",
        head.replace('\n', " ")
    );
    panic!(
        "{what} failed, stopped without retry; marker {}",
        ctx.marker
    );
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_credit_rounding() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the credit rounding probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-RND-{ts}"));
    verify_test_account(&ctx).await;

    // Read-only inputs.
    let methods = ctx.c.list_payment_methods().await.unwrap();
    let pm = methods
        .iter()
        .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
        .expect("a payment method with electronicInvoiceId 68")
        .id;
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    eprintln!(
        "SETUP contact {} payment method {}",
        customer.contact.id.get(),
        pm.get()
    );

    // 1. The net invoice: draft, finalize, read back.
    let note = format!("{}-invoice", ctx.marker);
    let spec = InvoiceSpec {
        customer: &customer,
        person,
        note,
        b2b: true,
        payment_method: Some(pm),
        with_address: true,
    };
    let saved = match ctx
        .c
        .save_invoice(&SaveInvoice::new(invoice_draft(&spec), positions()))
        .await
    {
        Ok(s) => s,
        Err(e) => reread_and_stop(&ctx, "Invoice", "save_invoice", &e).await,
    };
    let invoice_id = saved.invoice.id;
    eprintln!("CREATED Invoice {}", invoice_id.get());
    if let Err(e) = ctx.c.send_invoice_by(invoice_id, &SendBy::FINALIZE).await {
        // The finalize may have been written: show the document, then stop.
        let _ = read_sums(
            &ctx,
            "Invoice",
            invoice_id.get(),
            "invoice_after_failed_send",
        )
        .await;
        panic!("send_invoice_by failed ({e}), stopped without retry");
    }
    let invoice = read_sums(&ctx, "Invoice", invoice_id.get(), "invoice_final")
        .await
        .expect("the finalized invoice reads back");
    print_round("Invoice", "final", &invoice);

    // 2. The full credit note draft of that invoice, read back.
    let number = ctx.c.next_credit_note_number().await.unwrap();
    let note = format!("{}-credit-note", ctx.marker);
    let draft = {
        let mut draft = CreditNoteDraft {
            credit_note_number: number,
            contact: ObjectRef::new(customer.contact.id),
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
        draft.options.ref_src_invoice = Some(ObjectRef::new(invoice_id));
        draft.options.address_name = Some(customer.name.clone());
        draft.options.address_street = Some(customer.street.into());
        draft.options.address_zip = Some(customer.zip.into());
        draft.options.address_city = Some(customer.city.into());
        draft.options.address_country = Some(ObjectRef::new(GERMANY));
        draft.options.payment_method = Some(ObjectRef::new(pm));
        draft.options.is_e_invoice = true;
        draft
    };
    let saved = match ctx
        .c
        .save_credit_note(&SaveCreditNote::new(draft, positions()))
        .await
    {
        Ok(s) => s,
        Err(e) => reread_and_stop(&ctx, "CreditNote", "save_credit_note", &e).await,
    };
    let credit_id = saved.credit_note.id;
    eprintln!("CREATED CreditNote {}", credit_id.get());
    let credit_draft = read_sums(&ctx, "CreditNote", credit_id.get(), "credit_draft")
        .await
        .expect("the credit note draft reads back");
    print_round("CreditNote", "draft", &credit_draft);

    // 3. Finalize the credit note and read it back.
    if let Err(e) = ctx
        .c
        .send_credit_note_by(credit_id, &SendBy::FINALIZE)
        .await
    {
        let _ = read_sums(
            &ctx,
            "CreditNote",
            credit_id.get(),
            "credit_after_failed_send",
        )
        .await;
        panic!("send_credit_note_by failed ({e}), stopped without retry");
    }
    let credit_final = read_sums(&ctx, "CreditNote", credit_id.get(), "credit_final")
        .await
        .expect("the finalized credit note reads back");
    print_round("CreditNote", "final", &credit_final);

    eprintln!(
        "ROUND-VERDICT\tcredit gross {} vs invoice gross {}: equal={}\t\
         invoice tax: {}\tcredit draft tax: {}\tcredit final tax: {}\t\
         credit draft vs final sums equal={} (net {}/{}, tax {}/{}, gross {}/{}, positions {}/{})",
        credit_final.gross,
        invoice.gross,
        credit_final.gross == invoice.gross,
        tax_where(&invoice),
        tax_where(&credit_draft),
        tax_where(&credit_final),
        (
            &credit_draft.net,
            &credit_draft.tax,
            &credit_draft.gross,
            &credit_draft.positions
        ) == (
            &credit_final.net,
            &credit_final.tax,
            &credit_final.gross,
            &credit_final.positions
        ),
        credit_draft.net,
        credit_final.net,
        credit_draft.tax,
        credit_final.tax,
        credit_draft.gross,
        credit_final.gross,
        credit_draft.positions_text(),
        credit_final.positions_text(),
    );
}
