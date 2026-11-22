//! The field probe: for every ADDITIONAL writable field the spec names (not
//! sent by the crate today), does sevDesk STORE it, IGNORE it or REFUSE it, and
//! how do the STATE fields (`status`, `sendDate`, `payDate`, `sendType`,
//! `invoiceNumber`, contact `status`) behave. Writes PERMANENT objects to the
//! sevDesk TEST account (the probe aborts on any other account before the first
//! write), so it is its own test target (`test = false` in Cargo.toml) and runs
//! only on request:
//!
//! `just test-live --test field_probe case1`
//!
//! Method: ONE object per type is created with the crate's normal payload, then
//! updated field by field (only that field changes per write) and read back
//! (head and first position). The field list is the const table in `table.rs`.
//! Update routes: `PUT /Invoice/{id}`, `PUT /CreditNote/{id}`,
//! `PUT /Contact/{id}`, `PUT /ContactAddress/{id}`, `PUT /CommunicationWay/{id}`,
//! `PUT /CheckAccountTransaction/{id}`; voucher head and voucher, invoice and
//! credit note POSITION fields go through the save factory with the document id
//! and the position id (the factory must answer the same document, otherwise
//! the probe stops). Per object the order is: position fields, head fields,
//! state fields (`status` = 200 last: it may finalize and enshrine the document
//! and consume a number; that is the measurement).
//!
//! Budget: 7 permanent objects (invoice, credit note, voucher, contact, contact
//! address, communication way, transaction; limit 8) and at most 2 consumed
//! numbers (limit 3). Every object prints `CREATED <Object> <id>`, every
//! finalized document `NUMBER <Object> <id> <number>`. No write is retried; a
//! failed write is followed by a re-read.
//!
//! Output: `target/tmp/field_probe/<object>.tsv` (field, value sent, encoding,
//! class, read-back form, note), refused answers scrubbed in
//! `target/tmp/field_probe/captures/`, and stderr
//! `FIELD\t<object>\t<field>\t<class>\t<note>`.
//! Classes: `stored`, `ignored`, `refused`, `changed-other`, `same-value` (the
//! sent value equals the value before, nothing to tell), `untestable`.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness files are shared with live_write and use only some of the helpers; a failure in a test helper is a test failure"
)]

mod classify;
#[path = "../live_write/docs.rs"]
mod docs;
#[path = "../live_write/harness.rs"]
mod harness;
mod run;
#[path = "../common/scrub.rs"]
mod scrub;
mod table;
mod target;

use docs::*;
use harness::*;
use reqwest::Method;
use run::{Env, run_object};
use sevdesk::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use table::ROWS;
use target::{Factory, Target, objects_of};
use time::OffsetDateTime;
use time::macros::{date, datetime};

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;
/// The name prefix of the contacts the live tests create.
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";
const NET: i64 = 1000;
const CREDIT_NET: i64 = 500;

const PLANNED_OBJECTS: usize = 7;
const BUDGET_OBJECTS: usize = 8;
const PLANNED_NUMBERS: usize = 2;
const BUDGET_NUMBERS: usize = 3;
const _: () = assert!(PLANNED_OBJECTS <= BUDGET_OBJECTS && PLANNED_NUMBERS <= BUDGET_NUMBERS);

static CREATED: AtomicUsize = AtomicUsize::new(0);

/// Counts and prints a permanent object; stops before the write that would
/// exceed the budget (call `may_create` first).
fn created(what: &str, id: i64) {
    CREATED.fetch_add(1, Ordering::SeqCst);
    eprintln!("CREATED {what} {id}");
}

fn may_create() {
    assert!(
        CREATED.load(Ordering::SeqCst) < BUDGET_OBJECTS,
        "object budget exhausted"
    );
}

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

/// An EXISTING enshrined invoice (read-only list) that no cancellation invoice
/// cancels and that is big enough for the credit note.
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
    (*usable.first().unwrap_or_else(|| {
        panic!(
            "no existing enshrined, uncancelled invoice among {}: aborting before the write",
            all.len()
        )
    }))
    .clone()
}

/// The id of the first object of `GET /<name><query>` that is not `skip`;
/// read-only. `None` is reported and makes the fields that need it untestable.
async fn lookup(ctx: &Ctx, name: &str, query: &str, skip: Option<i64>) -> Option<i64> {
    let (status, text) = ctx
        .raw(Method::GET, &format!("/{name}{query}"), None, None)
        .await;
    let found = (status < 300)
        .then(|| objects_of(&text))
        .flatten()
        .and_then(|o| {
            o.as_array()?
                .iter()
                .filter_map(classify::id_i64)
                .find(|i| Some(*i) != skip)
        });
    eprintln!("LOOKUP\t{name}\tHTTP {status}\t{found:?}");
    found
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_writable_fields() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the field probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-FLD-{ts}"));
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
    let mut refs: BTreeMap<&'static str, i64> = BTreeMap::new();
    refs.insert("Contact", customer.contact.id.get());
    refs.insert("CheckAccount", BANK);
    for (name, query, skip) in [
        ("Part", "?limit=20", None),
        ("Order", "?limit=20", None),
        ("TaxSet", "?limit=20", None),
        ("CostCentre", "?limit=20", None),
        ("Document", "?limit=20", None),
        ("AccountingType", "?limit=20", None),
        ("Category", "?objectType=Contact&limit=100", Some(3)),
    ] {
        if let Some(id) = lookup(&ctx, name, query, skip).await {
            refs.insert(name, id);
        }
    }
    eprintln!(
        "SETUP contact {} payment method {} check account {} origin invoice {}",
        customer.contact.id.get(),
        pm.get(),
        check.id.get(),
        origin.id
    );
    eprintln!(
        "PLAN objects {PLANNED_OBJECTS} (budget {BUDGET_OBJECTS}), numbers at most {PLANNED_NUMBERS} (budget {BUDGET_NUMBERS})"
    );
    let env = Env { ctx: &ctx, refs };

    // 1. Invoice.
    may_create();
    let save = SaveInvoice::new(
        invoice_draft(&InvoiceSpec {
            customer: &customer,
            person,
            note: format!("{}-invoice", ctx.marker),
            b2b: true,
            payment_method: Some(pm),
            with_address: true,
        }),
        vec![net_position(1, NET, "SDRS-FLD Position", None)],
    );
    let mut base = serde_json::to_value(&save).unwrap();
    let saved = ctx.c.save_invoice(&save).await.unwrap();
    let id = saved.invoice.id.get();
    created("Invoice", id);
    base["invoice"]["id"] = id.into();
    base["invoicePosSave"][0]["id"] = saved
        .invoice_pos
        .first()
        .expect("a saved position")
        .id
        .get()
        .into();
    let target = Target {
        key: "invoice",
        object: "Invoice",
        id,
        head_get: format!("/Invoice/{id}"),
        pos_get: Some(format!(
            "/InvoicePos?invoice[id]={id}&invoice[objectName]=Invoice"
        )),
        put: Some(format!("/Invoice/{id}")),
        factory: Some(Factory {
            path: "/Invoice/Factory/saveInvoice",
            base,
            head_key: "invoice",
            pos_key: "invoicePosSave",
        }),
    };
    run_object(&env, &target, ROWS).await;

    // 2. Credit note.
    may_create();
    let c = customer_of(
        &ctx,
        origin
            .contact
            .expect("the origin invoice names a contact")
            .id,
    )
    .await;
    let note = format!("{}-credit-note", ctx.marker);
    let draft = CreditNoteDraft {
        credit_note_number: ctx.c.next_credit_note_number().await.unwrap(),
        contact: ObjectRef::new(c.contact.id),
        contact_person: ObjectRef::new(person),
        credit_note_date: date!(2026 - 10 - 01),
        delivery_date: datetime!(2026-10-01 0:00 +2),
        delivery_date_until: Some(datetime!(2026-10-31 0:00 +1)),
        header: note.clone(),
        head_text: None,
        foot_text: None,
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "Umsatzsteuer 19%".into(),
        booking_category: CreditNoteCategory::Underachievement,
        ref_src_invoice: Some(ObjectRef::new(origin.id)),
        currency: CurrencyCode::EUR,
        show_net: true,
        customer_internal_note: note,
        address_name: Some(c.name.clone()),
        address_street: Some(c.street.into()),
        address_zip: Some(c.zip.into()),
        address_city: Some(c.city.into()),
        address_country: Some(ObjectRef::new(GERMANY)),
        payment_method: Some(ObjectRef::new(pm)),
        is_e_invoice: true,
    };
    let save = SaveCreditNote::new(
        draft,
        vec![net_position(1, CREDIT_NET, "SDRS-FLD Position", None)],
    );
    let mut base = serde_json::to_value(&save).unwrap();
    let saved = ctx.c.save_credit_note(&save).await.unwrap();
    let id = saved.credit_note.id.get();
    created("CreditNote", id);
    base["creditNote"]["id"] = id.into();
    base["creditNotePosSave"][0]["id"] = saved
        .credit_note_pos
        .first()
        .expect("a saved position")
        .id
        .get()
        .into();
    let target = Target {
        key: "credit_note",
        object: "CreditNote",
        id,
        head_get: format!("/CreditNote/{id}"),
        pos_get: Some(format!(
            "/CreditNotePos?creditNote[id]={id}&creditNote[objectName]=CreditNote"
        )),
        put: Some(format!("/CreditNote/{id}")),
        factory: Some(Factory {
            path: "/CreditNote/Factory/saveCreditNote",
            base,
            head_key: "creditNote",
            pos_key: "creditNotePosSave",
        }),
    };
    run_object(&env, &target, ROWS).await;

    // 3. Voucher: head and positions both go through the factory.
    may_create();
    let saved = ctx
        .c
        .save_voucher(&SaveVoucher {
            voucher: ctx.draft(),
            positions: vec![position(35, "fee")],
            filename: None,
        })
        .await
        .unwrap();
    let id = saved.voucher.id.get();
    created("Voucher", id);
    let mut draft = ctx.draft();
    draft.id = Some(saved.voucher.id);
    let base = serde_json::to_value(SaveVoucher {
        voucher: draft,
        positions: with_ids(&saved, &[(35, "fee")]),
        filename: None,
    })
    .unwrap();
    let target = Target {
        key: "voucher",
        object: "Voucher",
        id,
        head_get: format!("/Voucher/{id}"),
        pos_get: Some(format!(
            "/VoucherPos?voucher[id]={id}&voucher[objectName]=Voucher"
        )),
        put: None,
        factory: Some(Factory {
            path: "/Voucher/Factory/saveVoucher",
            base,
            head_key: "voucher",
            pos_key: "voucherPosSave",
        }),
    };
    run_object(&env, &target, ROWS).await;

    // 4.-6. A person contact with an address and an e-mail address.
    may_create();
    let mut new = NewContact::new(ContactName::Person {
        surename: "SDRS-FLD".into(),
        familyname: format!("Probe {}", ctx.marker),
    });
    new.customer_number = Some(ctx.marker.clone());
    let contact = ctx.c.create_contact(&new).await.unwrap();
    let contact_id = contact.id.get();
    created("Contact", contact_id);
    may_create();
    let address = ctx
        .c
        .create_contact_address(&new_address(contact.id))
        .await
        .unwrap();
    created("ContactAddress", address.id.get());
    may_create();
    let way = ctx
        .c
        .create_communication_way(&new_email(contact.id, &ctx))
        .await
        .unwrap();
    created("CommunicationWay", way.id.get());
    // The fields that reference the own contact point at it, not at the
    // existing customer.
    let mut env = env;
    env.refs.insert("Contact", contact_id);
    let simple = |key, object: &'static str, id: i64| Target {
        key,
        object,
        id,
        head_get: format!("/{object}/{id}"),
        pos_get: None,
        put: Some(format!("/{object}/{id}")),
        factory: None,
    };
    // `parent` needs ANOTHER contact: the contact row runs with the existing one.
    let mut parent_env = Env {
        ctx: &ctx,
        refs: env.refs.clone(),
    };
    parent_env.refs.insert("Contact", customer.contact.id.get());
    run_object(&parent_env, &simple("contact", "Contact", contact_id), ROWS).await;
    run_object(
        &env,
        &simple("contact_address", "ContactAddress", address.id.get()),
        ROWS,
    )
    .await;
    run_object(
        &env,
        &simple("communication_way", "CommunicationWay", way.id.get()),
        ROWS,
    )
    .await;

    // 7. A transaction on the test check account.
    may_create();
    let tx = ctx
        .c
        .create_transaction(&ctx.new_transaction(ZEIT, 5000, datetime!(2026-11-05 12:00 +1)))
        .await
        .unwrap();
    created("CheckAccountTransaction", tx.id.get());
    run_object(
        &env,
        &simple("transaction", "CheckAccountTransaction", tx.id.get()),
        ROWS,
    )
    .await;
    eprintln!("DONE objects created: {}", CREATED.load(Ordering::SeqCst));
}
