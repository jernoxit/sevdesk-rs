//! Fixture decoding of payment methods, receipt guidance, sequences, users, uploads, tools, errors and the money edge cases.

use super::{client, fixture, sample_positions, serving};
use sevdesk::*;
use wiremock::matchers::{method, path};
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
    assert_eq!(
        list[0].translation_code.as_deref(),
        Some("PAYMENT_METHOD_58")
    );
    assert!(
        list[0]
            .text
            .as_deref()
            .is_some_and(|t| t.starts_with("Bitte"))
    );
    assert!(
        list.iter()
            .all(|m| m.create.is_some() && m.update.is_some())
    );
}

#[test]
fn sample_payment_method_list_reads_every_measured_form() {
    let list = sample_positions::<PaymentMethod>("sample_payment_method_list");
    assert_eq!(list.len(), 1);
    let m = &list[0];
    assert_eq!(m.id, PaymentMethodId::new(21919));
    assert_eq!(
        m.create,
        Some(time::macros::datetime!(2020-10-26 18:57:14 +1))
    );
    assert_eq!(
        m.update,
        Some(time::macros::datetime!(2024-10-28 22:20:24 +1))
    );
    assert_eq!(m.electronic_invoice_id, Some(58));
    assert_eq!(m.translation_code.as_deref(), Some("PAYMENT_METHOD_58"));
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
    assert_eq!(
        fees.description.as_deref(),
        Some("z.B. Kontogebühren und Gebühren für Kreditkarten")
    );
    assert_eq!(fees.allowed_receipt_types, vec![ReceiptType::Expense]);
    assert!(
        list.iter()
            .any(|g| g.allowed_receipt_types.contains(&ReceiptType::Revenue))
    );
    let rule = &fees.allowed_tax_rules[0];
    assert_eq!(rule.name.as_deref(), Some("VORST_ABZUGSF_AUFW"));
    assert_eq!(
        rule.description.as_deref(),
        Some("Vorsteuerabziehbare Aufwendungen")
    );
    assert_eq!(rule.tax_rates, ["ZERO", "SEVEN", "NINETEEN"]);
}

#[tokio::test]
async fn upload_temp_file_reads_every_scalar_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Voucher/Factory/uploadTempFile"))
        .respond_with(ResponseTemplate::new(201).set_body_string(fixture("write_upload_temp_file")))
        .mount(&server)
        .await;
    let file = client(&server)
        .upload_temp_file("a.pdf", "application/pdf", vec![1])
        .await
        .unwrap();
    assert_eq!(file.filename, "d97ad10ac85bdb2d863c4fa9af354997.pdf");
    assert_eq!(file.mime_type.as_deref(), Some("image/jpg"));
    assert_eq!(file.origin_mime_type.as_deref(), Some("application/pdf"));
    assert_eq!(file.pages, Some(1));
    assert_eq!(
        file.content_hash.as_deref(),
        Some("46620b3e7575523b425c72f4ffc21c023c2c084b660c8297ad3a5936bbe19716")
    );
    assert_eq!(file.is_e_invoice, Some(false));
}

#[test]
fn sample_sev_sequence_list_reads_every_measured_form() {
    let list = sample_positions::<SevSequence>("sample_sev_sequence_list");
    assert_eq!(list.len(), 1);
    let s = &list[0];
    assert_eq!(s.id, SevSequenceId::new(13449785));
    assert_eq!(s.for_object.as_deref(), Some("Invoice"));
    assert_eq!(s.kind.as_deref(), Some("RE"));
    assert_eq!(s.format.as_deref(), Some("RE-%NUMBER"));
    assert_eq!(s.next_sequence.as_deref(), Some("1274"));
    assert_eq!(s.next_number().as_deref(), Some("RE-1274"));
    assert_eq!(
        s.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert!(s.create.is_some() && s.update.is_some());
}

#[test]
fn sample_sev_user_list_reads_every_measured_form() {
    // SYNTHETIC fixture: built from the shape survey's forms (no live values,
    // the survey kept none because they are personal data).
    let list = sample_positions::<SevUser>("sample_sev_user_list");
    assert_eq!(list.len(), 1);
    let u = &list[0];
    assert_eq!(u.id, SevUserId::new(1));
    assert_eq!(u.status, Some(1));
    assert_eq!(u.username.as_deref(), Some("SDRS-SAMPLE"));
    assert_eq!(u.role.as_deref(), Some("SDRS-SAMPLE"));
    assert_eq!(u.sev_client, Some(ObjectRef::new(SevClientId::new(1))));
    let stamp = Some(time::macros::datetime!(2026-01-01 0:00 +1));
    assert_eq!(
        (u.create, u.update, u.last_login, u.last_password_change),
        (stamp, stamp, stamp, stamp)
    );
    for flag in [
        u.welcome_screen_seen,
        u.two_factor_auth,
        u.force_password_change,
        u.client_owner,
        u.default_receive_mail_copy,
        u.hide_maps_directions,
    ] {
        assert_eq!(flag, Some(false));
    }
    for text in [
        &u.fullname,
        &u.first_name,
        &u.last_name,
        &u.email,
        &u.gender,
        &u.member_code,
        &u.last_login_ip,
        &u.language_code,
    ] {
        assert_eq!(text.as_deref(), Some("SDRS-SAMPLE"));
    }
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
