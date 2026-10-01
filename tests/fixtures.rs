//! Every recorded fixture (real responses of the sevDesk test account, GET
//! only, headers not stored) read through the real client against a mock
//! server, plus the money edge cases.

mod common;

use common::{client, fixture};
use sevdesk::Amount;
use sevdesk::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn serving(route: &str, fixture_name: &str) -> (MockServer, SevdeskClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture(fixture_name)))
        .mount(&server)
        .await;
    let client = client(&server);
    (server, client)
}

fn eur(minor: i64) -> Amount {
    Amount::from_minor(minor)
}

#[tokio::test]
async fn check_account() {
    let (_s, c) = serving("/CheckAccount/6306017", "check_account").await;
    let a = c.check_account(CheckAccountId::new(6306017)).await.unwrap();
    assert_eq!(a.id, CheckAccountId::new(6306017));
    assert_eq!(a.name.as_deref(), Some("Probe Stripe Import PROBE-R1"));
    assert_eq!(a.kind.as_deref(), Some("online"));
    assert_eq!(a.accounting_number, Some(1201));
    assert_eq!(a.auto_map_transaction, Some(true));
    assert_eq!(a.balance, Some(eur(0)));
    assert_eq!(a.currency, CurrencyCode::EUR);
}

#[tokio::test]
async fn check_account_list() {
    let (_s, c) = serving("/CheckAccount", "check_account_list").await;
    let list = c.list_check_accounts().await.unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().any(|a| a.kind.as_deref() == Some("offline")));
    assert!(list.iter().all(|a| a.currency == CurrencyCode::EUR));
}

#[tokio::test]
async fn balance_at_date_reads_a_bare_negative_string() {
    let (_s, c) = serving("/CheckAccount/6306017/getBalanceAtDate", "balance_at_date").await;
    let date = time::macros::date!(2026 - 09 - 29);
    assert_eq!(
        c.balance_at_date(CheckAccountId::new(6306017), date)
            .await
            .unwrap(),
        eur(-139118)
    );
}

#[tokio::test]
async fn transaction_by_id() {
    let (_s, c) = serving("/CheckAccountTransaction/1897813785", "transaction").await;
    let t = c.transaction(TransactionId::new(1897813785)).await.unwrap();
    assert_eq!(t.amount, Some(eur(-1500)));
    assert_eq!(t.status, Some(TransactionStatus::Booked));
    assert_eq!(t.check_account.unwrap().id, CheckAccountId::new(6306017));
    assert_eq!(t.paymt_purpose.as_deref(), Some("probe diff B tx3"));
    assert!(!t.is_deleted());
    // The offset is kept: 12:00 Berlin, not UTC.
    assert_eq!(t.value_date.unwrap().offset().whole_hours(), 2);
}

#[tokio::test]
async fn deleted_transaction_by_id_is_marked_deleted() {
    let (_s, c) = serving("/CheckAccountTransaction/1897811012", "transaction_deleted").await;
    let t = c.transaction(TransactionId::new(1897811012)).await.unwrap();
    assert!(t.is_deleted());
    assert_eq!(t.status, Some(TransactionStatus::Created));
}

#[tokio::test]
async fn transaction_list() {
    let (_s, c) = serving("/CheckAccountTransaction", "transaction_list").await;
    let list = c
        .list_transactions(&TransactionQuery::new(CheckAccountId::new(6306017)))
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].amount, Some(eur(3451)));
    assert_eq!(list[0].status, Some(TransactionStatus::Linked));
}

#[tokio::test]
async fn transaction_log_reads_number_amount_and_document() {
    let (_s, c) = serving("/CheckAccountTransactionLog", "transaction_log").await;
    let log = c
        .transaction_log(TransactionId::new(1897813785))
        .await
        .unwrap();
    assert_eq!(log.len(), 1);
    // `amountPaid` is a JSON number (-15) here.
    assert_eq!(log[0].amount_paid, Some(eur(-1500)));
    assert_eq!(
        log[0].object,
        Some(BookedDocument::Voucher(VoucherId::new(156247137)))
    );
    assert_eq!(log[0].from_status, Some(TransactionStatus::Created));
    assert_eq!(log[0].to_status, Some(TransactionStatus::Booked));
}

#[tokio::test]
async fn voucher_with_number_paid_amount() {
    let (_s, c) = serving("/Voucher/156244013", "voucher").await;
    let v = c.voucher(VoucherId::new(156244013)).await.unwrap();
    assert_eq!(v.status, Some(VoucherStatus::Paid));
    assert_eq!(v.sum_gross, Some(eur(8749)));
    // `"paidAmount": 87.49` — a JSON number, read without a float.
    assert_eq!(v.paid_amount, Some(eur(8749)));
    assert_eq!(v.currency, CurrencyCode::EUR);
    assert_eq!(v.credit_debit, Some(CreditDebit::Credit));
    assert_eq!(v.voucher_type, Some(VoucherType::Normal));
    assert_eq!(v.tax_rule.unwrap().id, TaxRuleId::new(14));
    assert!(v.document.is_some());
    assert!(v.delivery_date_until.is_none());
    assert_eq!(
        v.supplier_name.as_deref(),
        Some("Stripe Payments Europe, Limited")
    );
}

#[tokio::test]
async fn voucher_positions() {
    let (_s, c) = serving("/VoucherPos", "voucher_positions").await;
    let p = c
        .voucher_positions(VoucherId::new(156244013))
        .await
        .unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].sum_net, Some(eur(5749)));
    // A glatter Betrag arrives as "30".
    assert_eq!(p[1].sum_net, Some(eur(3000)));
    assert_eq!(p[0].tax_rate_bp, Some(0));
    assert_eq!(p[0].account_datev.unwrap().id, AccountDatevId::new(4285));
    assert_eq!(p[0].net, Some(true));
}

#[tokio::test]
async fn voucher_list() {
    let (_s, c) = serving("/Voucher", "voucher_list").await;
    let list = c.list_vouchers(&VoucherQuery::default()).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].status, Some(VoucherStatus::PartiallyPaid));
    assert!(list.iter().all(|v| v.currency == CurrencyCode::EUR));
}

#[tokio::test]
async fn invoice_list_with_cancellation_pointing_to_origin() {
    let (_s, c) = serving("/Invoice", "invoice_list").await;
    let list = c.list_invoices(&InvoiceQuery::default()).await.unwrap();
    let sr = list
        .iter()
        .find(|i| i.invoice_type == Some(InvoiceType::Cancellation))
        .unwrap();
    assert_eq!(sr.invoice_number.as_deref(), Some("RE-1018"));
    assert_eq!(sr.origin.unwrap().id, InvoiceId::new(135171691));
    assert_eq!(sr.sum_gross, Some(eur(-4665)));
    assert_eq!(sr.currency, CurrencyCode::EUR);
    assert_eq!(sr.status, Some(InvoiceStatus::Paid));
    let re = list
        .iter()
        .find(|i| i.id == InvoiceId::new(135171691))
        .unwrap();
    assert_eq!(re.invoice_type, Some(InvoiceType::Invoice));
    assert!(re.origin.is_none());
    assert_eq!(re.customer_internal_note, sr.customer_internal_note);
}

#[tokio::test]
async fn payment_method_list_reads_the_electronic_invoice_id_from_a_string() {
    let (_s, c) = serving("/PaymentMethod", "payment_method_list").await;
    let list = c.list_payment_methods().await.unwrap();
    assert_eq!(list.len(), 6);
    let online = list
        .iter()
        .find(|m| m.id == PaymentMethodId::new(114251))
        .unwrap();
    assert_eq!(online.name.as_deref(), Some("Online-Zahlung (z.B. PayPal)"));
    assert_eq!(online.electronic_invoice_id, Some(68));
    assert_eq!(list[0].name.as_deref(), Some("SEPA Überweisung"));
    assert_eq!(list[0].electronic_invoice_id, Some(58));
}

#[tokio::test]
async fn invoice_draft_has_update_and_no_enshrined() {
    let (_s, c) = serving("/Invoice/135169163", "invoice_draft").await;
    let i = c.invoice(InvoiceId::new(135169163)).await.unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Draft));
    assert_eq!(i.invoice_number, None);
    assert_eq!(
        i.update,
        Some(time::macros::datetime!(2026-09-28 19:39:15 +2))
    );
    assert_eq!(i.enshrined, None);
    assert_eq!(i.sum_net, Some(eur(2900)));
    assert_eq!(i.sum_gross, Some(eur(3451)));
    assert_eq!(
        i.delivery_date,
        Some(time::macros::datetime!(2026-09-01 0:00 +2))
    );
    assert_eq!(
        i.delivery_date_until,
        Some(time::macros::datetime!(2026-09-30 0:00 +2))
    );
}

#[tokio::test]
async fn enshrined_invoice_has_number_update_and_enshrined() {
    let (_s, c) = serving("/Invoice/135169163", "invoice_enshrined").await;
    let i = c.invoice(InvoiceId::new(135169163)).await.unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Open));
    assert_eq!(i.invoice_number.as_deref(), Some("RE-1000"));
    assert_eq!(
        i.update,
        Some(time::macros::datetime!(2026-09-28 19:39:19 +2))
    );
    assert_eq!(
        i.enshrined,
        Some(time::macros::datetime!(2026-09-28 19:39:19 +2))
    );
    assert_eq!(i.currency, CurrencyCode::EUR);
}

#[tokio::test]
async fn credit_note() {
    let (_s, c) = serving("/CreditNote/2996041", "credit_note").await;
    let n = c.credit_note(CreditNoteId::new(2996041)).await.unwrap();
    assert_eq!(n.credit_note_number.as_deref(), Some("GU-1006"));
    assert_eq!(n.sum_gross, Some(eur(5831)));
    assert_eq!(n.currency, CurrencyCode::EUR);
    assert_eq!(n.status, Some(CreditNoteStatus::Paid));
    assert_eq!(n.origin.unwrap().id, InvoiceId::new(135171965));
}

#[tokio::test]
async fn receipt_guidance() {
    let (_s, c) = serving("/ReceiptGuidance/forExpense", "receipt_guidance").await;
    let list = c.receipt_guidance_for_expense().await.unwrap();
    let fees = list
        .iter()
        .find(|g| g.account_datev_id == AccountDatevId::new(4285))
        .unwrap();
    assert_eq!(fees.account_number.as_deref(), Some("4970"));
    assert!(
        fees.allowed_tax_rules
            .iter()
            .any(|r| r.id == TaxRuleId::new(14))
    );
}

#[tokio::test]
async fn bookkeeping_system_version() {
    let (_s, c) = serving(
        "/Tools/bookkeepingSystemVersion",
        "bookkeeping_system_version",
    )
    .await;
    assert_eq!(c.bookkeeping_system_version().await.unwrap(), "2.0");
}

#[tokio::test]
async fn error_404_code_151_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string(fixture("error_404_code_151")))
        .mount(&server)
        .await;
    let err = client(&server)
        .transaction(TransactionId::new(1))
        .await
        .unwrap_err();
    assert!(matches!(err, SevdeskError::NotFound { .. }), "{err:?}");
    assert_eq!(err.known_code(), Some(KnownCode::NotFound));
}

#[tokio::test]
async fn error_422_offline_account() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(422).set_body_string(fixture("error_422_offline_account")),
        )
        .mount(&server)
        .await;
    let err = client(&server)
        .create_transaction(&sample_transaction())
        .await
        .unwrap_err();
    assert!(err.is_offline_account(), "{err:?}");
    assert!(matches!(
        err,
        SevdeskError::Api {
            status: 422,
            code: None,
            ..
        }
    ));
}

pub fn sample_transaction() -> NewTransaction {
    NewTransaction {
        value_date: time::macros::datetime!(2026-08-15 12:00 +2),
        entry_date: None,
        amount: eur(-100),
        payee_payer_name: "Stripe Payments Europe, Limited".into(),
        paymt_purpose: Some("x".into()),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(CheckAccountId::new(6306016)),
        source_transaction: None,
        target_transaction: None,
    }
}

#[test]
fn amounts_read_exactly() {
    for (text, cents) in [
        ("30", 3000),
        ("\"30\"", 3000),
        ("57.49", 5749),
        ("\"57.49\"", 5749),
        ("-100.32", -10032),
        ("\"-100.32\"", -10032),
        ("87.49", 8749),
        ("\"-0.35\"", -35),
        ("\"-0.350\"", -35),
        ("0", 0),
        ("\"0.00\"", 0),
        ("-15", -1500),
        ("0.1", 10),
        ("1.0", 100),
    ] {
        assert_eq!(parse_amount(text).unwrap().minor(), cents, "{text}");
    }
    for bad in ["\"\"", "abc", "1e2", "1,5", "0.001", "--1", "\"1.2.3\""] {
        assert!(parse_amount(bad).is_err(), "{bad}");
    }
}

#[test]
fn amounts_write_as_exact_decimal_strings() {
    for (cents, text) in [
        (-5749, "-57.49"),
        (3000, "30.00"),
        (0, "0.00"),
        (-5, "-0.05"),
        (5, "0.05"),
        (-100032, "-1000.32"),
    ] {
        assert_eq!(format_decimal(Amount::from_minor(cents)), text);
    }
}

#[test]
fn scrub_collapses_expanded_account_objects() {
    use common::scrub::{collapse_account_objects, scrub_capture};
    let body = r#"{"objects":{"id":"1","sevClient":{"id":"7","objectName":"SevClient","name":"X","taxNumber":"1"},"list":[{"sevUser":{"id":"9","objectName":"SevUser","lastLogin":"now"}},{"sevUser":{"id":"9","objectName":"SevUser"}}]}}"#;
    let scrubbed: serde_json::Value = serde_json::from_str(&scrub_capture(body)).unwrap();
    assert_eq!(
        scrubbed["objects"]["sevClient"],
        serde_json::json!({"id": "7", "objectName": "SevClient"})
    );
    assert_eq!(
        scrubbed["objects"]["list"][0]["sevUser"],
        serde_json::json!({"id": "9", "objectName": "SevUser"})
    );
    assert_eq!(scrub_capture("not json"), "not json");
    // The recorded fixtures are clean.
    for name in ["write_contact_create", "write_invoice_cancel"] {
        let mut v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
        assert_eq!(collapse_account_objects(&mut v), 0, "{name}");
    }
}
