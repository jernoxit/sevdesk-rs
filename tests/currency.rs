//! The currency beside the amounts: required where the wire carries it, sent
//! exactly as given. Read variants are the recorded fixtures with `currency`
//! removed, nulled or replaced; write answers are synthetic.

#![allow(
    clippy::unwrap_used,
    reason = "test helper code outside #[test] functions: a failure here is a test failure"
)]

mod common;

use common::{client, credit_note_draft, eur, fixture, invoice_draft, position};
use serde_json::{Value, json};
use sevdesk::*;
use time::macros::date;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The fixture with `currency` of every object in `objects` set to `with`
/// (`None` removes the key).
fn fixture_with_currency(name: &str, with: Option<Value>) -> String {
    let mut body: Value = serde_json::from_str(&fixture(name)).unwrap();
    let objects = match &mut body["objects"] {
        Value::Array(list) => list.iter_mut().collect::<Vec<_>>(),
        one => vec![one],
    };
    for object in objects {
        let object = object.as_object_mut().unwrap();
        assert!(object.contains_key("currency"), "{name} carries a currency");
        match &with {
            Some(value) => object.insert("currency".into(), value.clone()),
            None => object.remove("currency"),
        };
    }
    body.to_string()
}

async fn serving(route: &str, body: String) -> (MockServer, SevdeskClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let c = client(&server);
    (server, c)
}

/// Reads every recorded object type that carries a currency, with the
/// fixture's `currency` set to `with`; the error of each read, if any.
async fn read_all(with: Option<Value>) -> Vec<(&'static str, Result<(), SevdeskError>)> {
    let body = |name| fixture_with_currency(name, with.clone());
    let mut results = Vec::new();
    let (_s, c) = serving("/CheckAccount/6306017", body("check_account")).await;
    let r = c.check_account(CheckAccountId::new(6306017)).await;
    results.push(("check_account", r.map(drop)));
    let (_s, c) = serving("/CheckAccount", body("check_account_list")).await;
    results.push((
        "check_account_list",
        c.list_check_accounts().await.map(drop),
    ));
    let (_s, c) = serving("/Voucher/156244013", body("voucher")).await;
    let r = c.voucher(VoucherId::new(156244013)).await;
    results.push(("voucher", r.map(drop)));
    let (_s, c) = serving("/Voucher", body("voucher_list")).await;
    let r = c.list_vouchers(&VoucherQuery::default()).await;
    results.push(("voucher_list", r.map(drop)));
    let (_s, c) = serving("/Invoice", body("invoice_list")).await;
    let r = c.list_invoices(&InvoiceQuery::default()).await;
    results.push(("invoice_list", r.map(drop)));
    let (_s, c) = serving("/CreditNote/2996041", body("credit_note")).await;
    let r = c.credit_note(CreditNoteId::new(2996041)).await;
    results.push(("credit_note", r.map(drop)));
    results
}

fn assert_currency_decode_error(results: Vec<(&str, Result<(), SevdeskError>)>) {
    for (name, result) in results {
        match result {
            Err(SevdeskError::Decode { source, .. }) => {
                assert!(source.to_string().contains("currency"), "{name}: {source}")
            }
            other => panic!("{name}: expected a decode error, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_missing_currency_is_a_decode_error_not_a_default() {
    assert_currency_decode_error(read_all(None).await);
}

#[tokio::test]
async fn a_null_currency_is_a_decode_error_not_a_default() {
    let results = read_all(Some(Value::Null)).await;
    for (name, result) in &results {
        assert!(
            matches!(result, Err(SevdeskError::Decode { .. })),
            "{name}: {result:?}"
        );
    }
}

#[tokio::test]
async fn a_malformed_currency_is_a_decode_error() {
    assert_currency_decode_error(read_all(Some(json!("eur"))).await);
}

#[tokio::test]
async fn an_unknown_but_well_formed_currency_is_kept() {
    for (name, result) in read_all(Some(json!("XYZ"))).await {
        assert!(result.is_ok(), "{name}: {result:?}");
    }
    let (_s, c) = serving(
        "/Invoice",
        fixture_with_currency("invoice_list", Some(json!("CHF"))),
    )
    .await;
    let list = c.list_invoices(&InvoiceQuery::default()).await.unwrap();
    assert!(list.iter().all(|i| i.currency.as_str() == "CHF"));
}

fn chf() -> CurrencyCode {
    CurrencyCode::new("CHF").unwrap()
}

async fn expecting_post(route: &str, sent: Value, answer: Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(route))
        .and(body_partial_json(sent))
        .respond_with(ResponseTemplate::new(201).set_body_json(answer))
        .expect(1)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn an_invoice_draft_sends_the_given_currency() {
    let server = expecting_post(
        "/Invoice/Factory/saveInvoice",
        json!({"invoice": {"currency": "CHF"}}),
        json!({"objects": {"invoice": {
            "id": "1", "status": "100", "currency": "CHF", "sumGross": "10"
        }}}),
    )
    .await;
    let draft = invoice_draft(chf());
    let position = position("Position", 1000);
    let saved = client(&server)
        .save_invoice(&SaveInvoice::new(draft, vec![position]))
        .await
        .unwrap();
    assert_eq!(saved.invoice.currency, chf());
}

#[tokio::test]
async fn a_credit_note_draft_sends_the_given_currency() {
    let server = expecting_post(
        "/CreditNote/Factory/saveCreditNote",
        json!({"creditNote": {"currency": "CHF"}}),
        json!({"objects": {"creditNote": {
            "id": "1", "status": "100", "currency": "CHF", "sumGross": "4"
        }}}),
    )
    .await;
    let draft = credit_note_draft("GU-1", chf());
    let position = position("Position", 400);
    let saved = client(&server)
        .save_credit_note(&SaveCreditNote::new(draft, vec![position]))
        .await
        .unwrap();
    assert_eq!(saved.credit_note.currency, chf());
}

#[tokio::test]
async fn a_voucher_draft_sends_the_given_currency() {
    let server = expecting_post(
        "/Voucher/Factory/saveVoucher",
        json!({"voucher": {"currency": "CHF"}}),
        json!({"objects": {"voucher": {
            "id": "1", "status": "50", "currency": "CHF", "sumGross": "57.49"
        }, "voucherPos": []}}),
    )
    .await;
    let voucher = VoucherDraft::new(
        date!(2026 - 08 - 31),
        "Supplier",
        "description",
        TaxRuleId::new(14),
        chf(),
    );
    let position = VoucherPositionDraft::new(AccountDatevId::new(4285), 0, eur(5749), "fees");
    let saved = client(&server)
        .save_voucher(&SaveVoucher {
            voucher,
            positions: vec![position],
            filename: None,
        })
        .await
        .unwrap();
    assert_eq!(saved.voucher.currency, chf());
}
