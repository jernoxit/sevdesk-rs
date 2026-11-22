//! The quantity probe: how does sevDesk return `quantity` and the prices of
//! positions saved with a fractional or zero quantity? Writes ONE PERMANENT
//! invoice draft to the sevDesk TEST account (the probe aborts on any other
//! account before the write), so it is its own test target (`test = false` in
//! Cargo.toml) and runs only on request:
//!
//! `just test-live --test quantity_probe case1`
//!
//! The draft is the crate's own invoice draft; only the position list is
//! replaced by probe-local positions that the crate's `DocumentPositionDraft`
//! cannot express (a fractional quantity, a price with four decimals):
//!
//! | pos | quantity (JSON number) | price (JSON string) |
//! |-----|------------------------|---------------------|
//! | 1   | `0`                    | `"10"`              |
//! | 2   | `0.5`                  | `"12.3456"`         |
//! | 3   | `1`                    | `"10"`              |
//! | 4   | `2`                    | `"10"`              |
//!
//! The customer is an EXISTING test contact (read-only lookup, verified by
//! GET). Exactly one write: the save. Everything else is GET.
//!
//! Output: raw scrubbed bodies in `target/tmp/captures/quantity_probe_<what>.json`
//! and one stderr line per position and read,
//! `QUANTITY\t<read>\t<pos>\t<quantity>\t<price>\t<priceNet>\t<priceGross>\t<priceTax>\t<sumNet>\t<sumGross>`,
//! with the RAW JSON text of each value (`"1"` vs `1` stays visible), plus
//! `QUANTITY_HEAD\t<read>\t…` lines with the invoice head sums.
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
use serde::Serialize;
use serde_json::Number;
use serde_json::value::RawValue;
use sevdesk::*;
use std::collections::BTreeMap;
use std::str::FromStr;
use time::OffsetDateTime;

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;
/// The name prefix of the contacts the live tests create.
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";

/// A position as `PositionWire` (src/document_payload/position.rs) sends it,
/// except `quantity` is an exact JSON number and `price` a string as given.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbePosition {
    object_name: &'static str,
    map_all: bool,
    quantity: Number,
    price: &'static str,
    name: String,
    unity: ObjectRef<UnityId>,
    #[serde(rename = "taxRate")]
    tax_rate: &'static str,
}

/// quantity text, price text, name suffix.
const POSITIONS: [(&str, &str, &str); 4] = [
    ("0", "10", "qty 0"),
    ("0.5", "12.3456", "qty 0.5"),
    ("1", "10", "qty 1"),
    ("2", "10", "qty 2"),
];

type Raw = BTreeMap<String, Box<RawValue>>;

#[derive(serde::Deserialize)]
struct Envelope {
    objects: Box<RawValue>,
}

fn raw_text(m: &Raw, key: &str) -> String {
    m.get(key)
        .map_or_else(|| "<absent>".into(), |v| v.get().to_owned())
}

/// The objects of an answer: a list, or one object as a one-element list.
fn objects_raw(body: &str) -> Vec<Raw> {
    let env: Envelope =
        serde_json::from_str(body).unwrap_or_else(|e| panic!("answer has no objects: {e}"));
    let text = env.objects.get();
    if text.trim_start().starts_with('[') {
        serde_json::from_str(text).unwrap()
    } else {
        vec![serde_json::from_str(text).unwrap()]
    }
}

/// The positions inside an answer: a list of objects, or an object holding a
/// key that contains `pos` and a list (the save answer's shape).
fn positions_raw(body: &str) -> Vec<Raw> {
    let objects = objects_raw(body);
    if objects.first().is_some_and(|o| o.contains_key("quantity")) {
        return objects;
    }
    for o in &objects {
        for (k, v) in o {
            if k.to_lowercase().contains("pos") && v.get().trim_start().starts_with('[') {
                return serde_json::from_str(v.get()).unwrap();
            }
        }
    }
    eprintln!(
        "QUANTITY-NOTE\tno positions found; top-level keys: {:?}",
        objects
            .first()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
    );
    Vec::new()
}

fn record_positions(read: &str, body: &str) {
    for (i, p) in positions_raw(body).iter().enumerate() {
        let cols: Vec<String> = [
            "quantity",
            "price",
            "priceNet",
            "priceGross",
            "priceTax",
            "sumNet",
            "sumGross",
        ]
        .iter()
        .map(|k| raw_text(p, k))
        .collect();
        eprintln!(
            "QUANTITY\t{read}\t{}\t{}\t{}",
            raw_text(p, "name").replace('\t', " ") + &format!(" (#{i})"),
            cols[0],
            cols[1..].join("\t")
        );
    }
}

fn record_head(read: &str, body: &str) {
    for o in objects_raw(body) {
        let sums: Vec<String> = ["sumNet", "sumTax", "sumGross", "sumNetAccounting"]
            .iter()
            .map(|k| format!("{k}={}", raw_text(&o, k)))
            .collect();
        eprintln!(
            "QUANTITY_HEAD\t{read}\t{}\t{}",
            raw_text(&o, "id"),
            sums.join("\t")
        );
    }
}

/// A GET that must answer below 300; the scrubbed body is saved under `what`.
async fn get(ctx: &Ctx, path: &str, what: &str) -> String {
    let (status, body) = ctx.raw(Method::GET, path, None, Some(what)).await;
    assert!(
        status < 300,
        "GET {path} answered {status}: {}",
        scrub::scrub_capture(&body.chars().take(2000).collect::<String>())
    );
    body
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
    let verified = ctx.c.contact(contact.id).await.unwrap();
    assert_eq!(verified.id, contact.id);
    let name = verified.name.clone().expect("the contact has a name");
    eprintln!("SETUP contact {}", contact.id.get());
    Customer {
        contact: verified,
        name,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

/// A failed save: the half-written check by `customerInternalNote`.
async fn half_written(ctx: &Ctx, note: &str) {
    let (status, body) = ctx
        .raw(
            Method::GET,
            &format!("/Invoice?customerInternalNote={note}"),
            None,
            Some("quantity_probe_half_written"),
        )
        .await;
    eprintln!(
        "QUANTITY-NOTE\thalf_written\tHTTP {status}\t{}",
        scrub::scrub_capture(&body.chars().take(2000).collect::<String>())
    );
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_quantities_and_prices() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the quantity probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let marker = format!("SDRS-QTY-{ts}");
    let ctx = Ctx::new(&token, marker.clone());
    verify_test_account(&ctx).await;

    let methods = ctx.c.list_payment_methods().await.unwrap();
    let pm = methods
        .iter()
        .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
        .expect("a payment method with electronicInvoiceId 68")
        .id;
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    let note = format!("{marker}-quantity");

    // The crate's own draft, serialized; only the position list is replaced.
    let draft = invoice_draft(&InvoiceSpec {
        customer: &customer,
        person,
        note: note.clone(),
        b2b: true,
        payment_method: Some(pm),
        with_address: true,
    });
    let mut body = serde_json::to_value(SaveInvoice::new(draft, Vec::new())).unwrap();
    let positions: Vec<ProbePosition> = POSITIONS
        .iter()
        .map(|(quantity, price, suffix)| ProbePosition {
            object_name: "InvoicePos",
            map_all: true,
            quantity: Number::from_str(quantity).unwrap(),
            price,
            name: format!("{marker} {suffix}"),
            unity: ObjectRef::new(PIECE),
            tax_rate: "19.00",
        })
        .collect();
    body["invoicePosSave"] = serde_json::to_value(&positions).unwrap();
    eprintln!("REQUEST positions\t{}", body["invoicePosSave"]);

    // The one write.
    let (status, answer) = ctx
        .raw(
            Method::POST,
            "/Invoice/Factory/saveInvoice",
            Some(body.to_string()),
            Some("quantity_probe_save"),
        )
        .await;
    let failed = status >= 300 || answer.contains("\"error\"");
    if failed {
        eprintln!(
            "QUANTITY-NOTE\tsave failed\tHTTP {status}\t{}",
            scrub::scrub_capture(&answer.chars().take(2000).collect::<String>())
        );
        half_written(&ctx, &note).await;
        panic!("the save failed (HTTP {status}); see QUANTITY-NOTE lines");
    }
    record_positions("save", &answer);

    let id = objects_raw(&answer)
        .first()
        .and_then(|o| {
            o.get("invoice")
                .map(|v| v.get().to_owned())
                .or_else(|| o.get("id").map(|v| v.get().to_owned()))
        })
        .and_then(|t| {
            serde_json::from_str::<serde_json::Value>(&t)
                .ok()
                .and_then(|v| {
                    let id = if v.is_object() { v["id"].clone() } else { v };
                    id.as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| id.as_i64())
                })
        })
        .expect("the save answer names the invoice id");
    eprintln!("CREATED Invoice {id}");

    let head = get(
        &ctx,
        &format!("/Invoice/{id}"),
        "quantity_probe_get_invoice",
    )
    .await;
    record_head("get", &head);
    let pos = get(
        &ctx,
        &format!("/InvoicePos?invoice[id]={id}&invoice[objectName]=Invoice"),
        "quantity_probe_get_invoice_pos",
    )
    .await;
    record_positions("get_pos", &pos);
    let list = get(
        &ctx,
        &format!("/Invoice?customerInternalNote={note}"),
        "quantity_probe_list",
    )
    .await;
    record_head("list", &list);
}
