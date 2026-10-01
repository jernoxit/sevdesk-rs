//! Helpers of the contact, invoice and credit note live tests. Their objects
//! carry the marker `SDRS-LIVE-<unix ts>-<test>`; an enshrined invoice cannot be
//! deleted, so nothing sweeps them: the run prints what it created.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helper code outside #[test] functions: a failure here is a test failure"
)]

use super::harness::*;
use reqwest::Method;
use serde::de::DeserializeOwned;
use sevdesk::*;
use std::future::Future;
use std::sync::Arc;
use time::OffsetDateTime;
use time::macros::{date, datetime};

pub(crate) const DOC_PREFIX: &str = "SDRS-LIVE-";
/// `electronicInvoiceId` of the payment method "Online-Zahlung".
pub(crate) const ONLINE_PAYMENT_XML_ID: i64 = 68;

#[derive(serde::Deserialize)]
struct Envelope<T> {
    objects: T,
}

/// The `objects` of a raw answer, decoded into one of the crate's types.
pub(crate) fn objects<T: DeserializeOwned>(text: &str) -> T {
    serde_json::from_str::<Envelope<T>>(text)
        .unwrap_or_else(|e| {
            panic!(
                "decode: {e}: {}",
                text.chars().take(300).collect::<String>()
            )
        })
        .objects
}

/// `objects` of a raw answer as untyped JSON, for facts the wire types do not
/// read (test assertions only).
pub(crate) fn raw_objects(text: &str) -> serde_json::Value {
    objects(text)
}

/// Like [`raw_objects`], with a one-element list unwrapped: sevDesk answers a
/// single object either way, depending on the call.
pub(crate) fn raw_one(text: &str) -> serde_json::Value {
    match raw_objects(text) {
        serde_json::Value::Array(mut list) if list.len() == 1 => list.remove(0),
        other => other,
    }
}

/// [`objects`] for a single object, also when it comes as a one-element list.
pub(crate) fn objects_one<T: DeserializeOwned>(text: &str) -> T {
    serde_json::from_value(raw_one(text)).unwrap()
}

pub(crate) fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

/// Asserts a 2xx answer and hands back the body.
pub(crate) fn ok((status, text): (u16, String)) -> String {
    assert!(
        status < 300,
        "{status} {}",
        text.chars().take(400).collect::<String>()
    );
    text
}

/// Like the harness's `run`, for the document tests: one at a time, a fresh
/// `SDRS-LIVE-` marker, no sweep (enshrined documents stay).
pub(crate) async fn run_docs<F, Fut>(test: &str, body: F)
where
    F: FnOnce(Arc<Ctx>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping live write test");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Arc::new(Ctx::new(&token, format!("{DOC_PREFIX}{ts}-{test}")));
    if let Err(e) = tokio::spawn(body(ctx)).await {
        match e.try_into_panic() {
            Ok(panic) => std::panic::resume_unwind(panic),
            Err(e) => panic!("test task failed: {e}"),
        }
    }
}

/// Cuts the long base64 `content` of a recorded `getPdf` answer (or the XML
/// string of `getXml`) down to a fixture size.
pub(crate) fn shorten_capture(name: &str) {
    let path = format!("{}/captures/{name}.json", env!("CARGO_TARGET_TMPDIR"));
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    // A multiple of four keeps the base64 decodable.
    let cut = |s: &str, keep: usize| s.chars().take(keep).collect::<String>();
    if let Some(content) = v["objects"]["content"].as_str().map(|s| cut(s, 400)) {
        v["objects"]["content"] = content.into();
    } else if let Some(xml) = v["objects"].as_str().map(|s| cut(s, 800)) {
        v["objects"] = xml.into();
    }
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
}

/// Writes bytes next to the captures, for a look with `pdftotext`.
pub(crate) fn keep_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let dir = format!("{}/captures", env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = std::path::PathBuf::from(format!("{dir}/{name}"));
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The text of a PDF when `pdftotext` is installed; `None` otherwise.
pub(crate) fn pdf_text(path: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// What the tests need of a created customer.
pub(crate) struct Customer {
    pub(crate) contact: Contact,
    pub(crate) name: String,
    pub(crate) street: &'static str,
    pub(crate) zip: &'static str,
    pub(crate) city: &'static str,
}

pub(crate) const STREET: &str = "Teststrasse 1";
pub(crate) const ZIP: &str = "10115";
pub(crate) const CITY: &str = "Berlin";

/// `Category` 47 "Rechnungsanschrift" of `ContactAddress`.
pub(crate) const BILLING_ADDRESS: AddressCategoryId = AddressCategoryId::new(47);
/// `CommunicationWayKey` 2 "Arbeit".
pub(crate) const WORK_KEY: CommunicationWayKeyId = CommunicationWayKeyId::new(2);

pub(crate) fn new_address(contact: ContactId) -> NewContactAddress {
    NewContactAddress {
        contact: ObjectRef::new(contact),
        street: STREET.into(),
        zip: ZIP.into(),
        city: CITY.into(),
        country: ObjectRef::new(GERMANY),
        category: ObjectRef::new(BILLING_ADDRESS),
    }
}

pub(crate) fn new_email(contact: ContactId, ctx: &Ctx) -> NewCommunicationWay {
    let local: String = ctx
        .marker
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    NewCommunicationWay::email(
        contact,
        format!("{}@example.com", local.to_lowercase()),
        WORK_KEY,
        true,
    )
}

/// A B2B organisation with buyer reference "-", an own customer number, an
/// address and an e-mail address. `record` stores the three create answers as
/// fixtures. Created through the raw calls (typed payloads), so the bodies can
/// be recorded.
pub(crate) async fn b2b_customer(ctx: &Ctx, record: bool) -> Customer {
    let rec = |name: &'static str| record.then_some(name);
    let name = format!("SDRS-LIVE Testkunde GmbH {}", ctx.marker);
    let mut new = NewContact::new(ContactName::Organisation { name: name.clone() });
    new.buyer_reference = Some("-".into());
    new.customer_number = Some(ctx.marker.clone());
    let contact: Contact = objects_one(&ok(ctx
        .raw(
            Method::POST,
            "/Contact",
            Some(json(&new)),
            rec("write_contact_create"),
        )
        .await));
    assert_eq!(
        contact.customer_number.as_deref(),
        Some(ctx.marker.as_str())
    );
    assert_eq!(contact.buyer_reference.as_deref(), Some("-"));
    ok(ctx
        .raw(
            Method::POST,
            "/ContactAddress",
            Some(json(&new_address(contact.id))),
            rec("write_contact_address_create"),
        )
        .await);
    ok(ctx
        .raw(
            Method::POST,
            "/CommunicationWay",
            Some(json(&new_email(contact.id, ctx))),
            rec("write_communication_way_create"),
        )
        .await);
    Customer {
        contact,
        name,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

/// A B2C person (`surename`/`familyname`, no buyer reference) with address and
/// e-mail, created through the typed calls.
pub(crate) async fn b2c_customer(ctx: &Ctx) -> Customer {
    let new = NewContact::new(ContactName::Person {
        surename: "SDRS-LIVE".into(),
        familyname: format!("Privatkunde {}", ctx.marker),
    });
    let contact = ctx.c.create_contact(&new).await.unwrap();
    ctx.c
        .create_contact_address(&new_address(contact.id))
        .await
        .unwrap();
    ctx.c
        .create_communication_way(&new_email(contact.id, ctx))
        .await
        .unwrap();
    Customer {
        name: format!("SDRS-LIVE Privatkunde {}", ctx.marker),
        contact,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

/// The user a document names as contact person.
pub(crate) async fn contact_person(ctx: &Ctx) -> SevUserId {
    ctx.c.list_sev_users().await.unwrap()[0].id
}

pub(crate) fn net_position(
    quantity: i64,
    minor: i64,
    name: &str,
    text: Option<&str>,
) -> DocumentPositionDraft {
    DocumentPositionDraft {
        quantity,
        price: eur(minor),
        name: name.into(),
        text: text.map(Into::into),
        unity: PIECE,
        tax_rate_bp: 1900,
    }
}

pub(crate) struct InvoiceSpec<'a> {
    pub(crate) customer: &'a Customer,
    pub(crate) person: SevUserId,
    pub(crate) note: String,
    pub(crate) b2b: bool,
    pub(crate) payment_method: Option<PaymentMethodId>,
    pub(crate) with_address: bool,
}

/// An invoice draft dated 2026-10-01 with the service period 2026-10-01 to
/// 2026-10-31 (Berlin midnights, CEST then CET).
pub(crate) fn invoice_draft(s: &InvoiceSpec<'_>) -> InvoiceDraft {
    let addr = |v: &str| s.with_address.then(|| v.to_owned());
    InvoiceDraft {
        contact: ObjectRef::new(s.customer.contact.id),
        contact_person: ObjectRef::new(s.person),
        invoice_date: date!(2026 - 10 - 01),
        delivery_date: datetime!(2026-10-01 0:00 +2),
        delivery_date_until: Some(datetime!(2026-10-31 0:00 +1)),
        header: s.note.clone(),
        head_text: None,
        foot_text: None,
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "Umsatzsteuer 19%".into(),
        currency: CurrencyCode::EUR,
        show_net: s.b2b,
        time_to_pay: 14,
        customer_internal_note: s.note.clone(),
        address_name: addr(&s.customer.name),
        address_street: addr(s.customer.street),
        address_zip: addr(s.customer.zip),
        address_city: addr(s.customer.city),
        address_country: s.with_address.then(|| ObjectRef::new(GERMANY)),
        payment_method: s.payment_method.map(ObjectRef::new),
        is_e_invoice: s.b2b,
    }
}

/// Prints a measured fact where the log filter `MEASURED` finds it.
pub(crate) fn measured(what: &str) {
    eprintln!("MEASURED {what}");
}
