//! Fixture decoding of payment methods, receipt guidance, tools, errors and the money edge cases.

use super::{client, fixture, serving};
use sevdesk::*;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

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
    use super::common::scrub::{collapse_account_objects, scrub_capture};
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
