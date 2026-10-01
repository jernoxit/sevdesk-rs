//! Outgoing request payloads and answers against a mock server. Write answers
//! are real captures (`fixtures/write_*.json`, recorded by
//! `live_write_6_record_raw_write_bodies`); bodies marked "synthetic" are built
//! from the measured protocol instead.

mod common;

use common::{client, eur, fixture};
use serde_json::json;
use sevdesk::*;
use time::macros::{date, datetime};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn transaction_list_sends_the_measured_filter_keys() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/CheckAccountTransaction"))
        .and(query_param("checkAccount[id]", "6306017"))
        .and(query_param("checkAccount[objectName]", "CheckAccount"))
        .and(query_param("paymtPurpose", "txn_1"))
        .and(query_param("startDate", "1785499200"))
        .and(query_param("isBooked", "true"))
        .and(query_param("status", "100"))
        .and(query_param("limit", "1000"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("transaction_list")))
        .expect(1)
        .mount(&server)
        .await;
    let mut q = TransactionQuery::new(CheckAccountId::new(6306017));
    q.paymt_purpose = Some("txn_1".into());
    q.start_date = Some(datetime!(2026-07-31 12:00 UTC));
    q.is_booked = Some(true);
    q.status = Some(TransactionStatus::Created);
    q.page = Some(Page::new(1000, 0).unwrap());
    client(&server).list_transactions(&q).await.unwrap();
}

#[test]
fn page_limit_is_bounded() {
    assert!(Page::new(0, 0).is_err());
    assert!(Page::new(1001, 0).is_err());
    assert!(Page::new(1, 0).is_ok());
    assert!(Page::new(1000, 5).is_ok());
}

#[tokio::test]
async fn invoice_filter_uses_the_correct_spelling() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice"))
        .and(query_param("customerInternalNote", "PROBE-R1-E3-pos-pct"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("invoice_list")))
        .expect(1)
        .mount(&server)
        .await;
    let q = InvoiceQuery {
        customer_internal_note: Some("PROBE-R1-E3-pos-pct".into()),
        ..Default::default()
    };
    assert_eq!(client(&server).list_invoices(&q).await.unwrap().len(), 2);
}

#[tokio::test]
async fn voucher_filter_keys() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Voucher"))
        .and(query_param("descriptionLike", "M11"))
        .and(query_param("status", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("voucher_list")))
        .expect(1)
        .mount(&server)
        .await;
    let q = VoucherQuery {
        description_like: Some("M11".into()),
        status: Some(VoucherStatus::Draft),
        page: None,
    };
    client(&server).list_vouchers(&q).await.unwrap();
}

#[tokio::test]
async fn save_voucher_update_sends_every_position_with_its_id_and_exact_amounts() {
    let server = MockServer::start().await;
    let expected = json!({
        "voucher": {
            "objectName": "Voucher", "mapAll": true, "id": 156244013,
            "voucherDate": "31.08.2026", "deliveryDate": "31.08.2026",
            "supplierName": "Stripe Payments Europe, Limited",
            "description": "7032257561", "status": 50, "creditDebit": "C",
            "voucherType": "VOU", "taxRule": {"id": 14, "objectName": "TaxRule"},
            "currency": "EUR"
        },
        "voucherPosSave": [
            {"objectName": "VoucherPos", "mapAll": true, "id": 213039458,
             "accountDatev": {"id": 4285, "objectName": "AccountDatev"},
             "taxRate": "0.00", "net": true, "sumNet": "57.49", "comment": "fees"},
            {"objectName": "VoucherPos", "mapAll": true, "id": 213039459,
             "accountDatev": {"id": 4285, "objectName": "AccountDatev"},
             "taxRate": "0.00", "net": true, "sumNet": "30.00", "comment": "disputes"}
        ],
        "filename": "abc.pdf"
    });
    // A real answer of the test account (2 positions, PDF attached, released).
    let answer = fixture("write_voucher_save_update_release_attach");
    Mock::given(method("POST"))
        .and(path("/Voucher/Factory/saveVoucher"))
        .and(body_json(expected))
        .respond_with(ResponseTemplate::new(201).set_body_string(answer))
        .expect(1)
        .mount(&server)
        .await;
    let mut voucher = VoucherDraft::new(
        date!(2026 - 08 - 31),
        "Stripe Payments Europe, Limited",
        "7032257561",
        TaxRuleId::new(14),
        CurrencyCode::EUR,
    );
    voucher.id = Some(VoucherId::new(156244013));
    voucher.delivery_date = Some(date!(2026 - 08 - 31));
    let position = |id: i64, cents: i64, comment: &str| {
        VoucherPositionDraft::new(AccountDatevId::new(4285), 0, eur(cents), comment)
            .with_id(VoucherPositionId::new(id))
    };
    let saved = client(&server)
        .save_voucher(&SaveVoucher {
            voucher,
            positions: vec![
                position(213039458, 5749, "fees"),
                position(213039459, 3000, "disputes"),
            ],
            filename: Some("abc.pdf".into()),
        })
        .await
        .unwrap();
    assert_eq!(saved.voucher_pos.len(), 2);
    assert_eq!(saved.voucher.sum_gross, Some(eur(45)));
    assert_eq!(saved.voucher.status, Some(VoucherStatus::Open));
    assert_eq!(saved.voucher.currency, CurrencyCode::EUR);
    assert!(saved.voucher.document.is_some());
}

#[tokio::test]
async fn book_amounts_carry_the_transactions_sign_and_endpoint_specific_dates() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/Voucher/156244013/bookAmount"))
        .and(body_json(json!({
            "amount": "-57.49", "date": "2026-08-25T12:00:00+02:00", "type": "N",
            "checkAccount": {"id": 6306017, "objectName": "CheckAccount"},
            "checkAccountTransaction": {"id": 1897813785, "objectName": "CheckAccountTransaction"}
        })))
        // A real answer: the booking is a `VoucherLog` object.
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("write_voucher_book_amount")),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/CreditNote/2996041/bookAmount"))
        .and(body_json(json!({
            "amount": "-58.31", "date": 1787652000, "type": "FULL_PAYMENT",
            "checkAccount": {"id": 6306017, "objectName": "CheckAccount"},
            "checkAccountTransaction": {"id": 1897813785, "objectName": "CheckAccountTransaction"}
        })))
        // Synthetic: credit note bookings were measured but not recorded; only the request matters.
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"objects":{}}"#))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    let account = ObjectRef::new(CheckAccountId::new(6306017));
    let transaction = ObjectRef::new(TransactionId::new(1897813785));
    c.book_voucher_amount(
        VoucherId::new(156244013),
        &BookVoucherAmount {
            amount: eur(-5749),
            date: datetime!(2026-08-25 12:00 +2),
            booking_type: BookingType::N,
            check_account: account,
            check_account_transaction: transaction,
        },
    )
    .await
    .unwrap();
    c.book_credit_note_amount(
        CreditNoteId::new(2996041),
        &BookDocumentAmount {
            amount: eur(-5831),
            date: datetime!(2026-08-25 12:00 +2),
            booking_type: BookingType::FullPayment,
            check_account: account,
            check_account_transaction: transaction,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn transaction_create_update_and_check_account_update_payloads() {
    let server = MockServer::start().await;
    let created = fixture("write_transaction_create");
    let updated = fixture("write_transaction_update");
    Mock::given(method("POST"))
        .and(path("/CheckAccountTransaction"))
        .and(body_json(json!({
            "valueDate": "2026-08-15T12:00:00+02:00", "amount": "-15.00",
            "payeePayerName": "Stripe Payments Europe, Limited",
            "paymtPurpose": "txn_1", "status": 100,
            "checkAccount": {"id": 6306017, "objectName": "CheckAccount"}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_string(created))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/CheckAccountTransaction/1897813785"))
        .and(body_json(json!({
            "status": 400,
            "targetTransaction": {"id": 1897813786, "objectName": "CheckAccountTransaction"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_string(updated))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/CheckAccount/6306017"))
        .and(body_json(json!({"autoMapTransaction": "0"})))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("check_account")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/CheckAccountTransaction/1897813785"))
        // A real answer: `objects` is a list holding one `null`.
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("write_transaction_delete")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    let new = NewTransaction {
        value_date: datetime!(2026-08-15 12:00 +2),
        amount: eur(-1500),
        payee_payer_name: "Stripe Payments Europe, Limited".into(),
        paymt_purpose: Some("txn_1".into()),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(CheckAccountId::new(6306017)),
        source_transaction: None,
        target_transaction: None,
    };
    c.create_transaction(&new).await.unwrap();
    c.update_transaction(
        TransactionId::new(1897813785),
        &TransactionUpdate {
            status: Some(TransactionStatus::Booked),
            target_transaction: Some(ObjectRef::new(TransactionId::new(1897813786))),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    c.update_check_account(
        CheckAccountId::new(6306017),
        &CheckAccountUpdate {
            auto_map_transaction: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    c.delete_transaction(TransactionId::new(1897813785))
        .await
        .unwrap();
}

#[tokio::test]
async fn upload_temp_file_is_multipart_field_file() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Voucher/Factory/uploadTempFile"))
        .and(wiremock::matchers::header_regex(
            "Content-Type",
            "^multipart/form-data",
        ))
        .and(wiremock::matchers::body_string_contains("name=\"file\""))
        .respond_with(ResponseTemplate::new(201).set_body_string(fixture("write_upload_temp_file")))
        .expect(1)
        .mount(&server)
        .await;
    let up = client(&server)
        .upload_temp_file("fee.pdf", "application/pdf", b"%PDF-1.4".to_vec())
        .await
        .unwrap();
    assert_eq!(up.filename, "d97ad10ac85bdb2d863c4fa9af354997.pdf");
    assert_eq!(up.pages, Some(1));
}

#[tokio::test]
async fn reset_and_delete_answers_decode_from_real_captures() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/Voucher/1/resetToOpen"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("write_voucher_reset_to_open")),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/Voucher/1/resetToDraft"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("write_voucher_reset_to_open")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    c.reset_voucher_to_open(VoucherId::new(1)).await.unwrap();
    c.reset_voucher_to_draft(VoucherId::new(1)).await.unwrap();
}

#[tokio::test]
async fn deleting_a_booked_transaction_is_a_409_with_code_159() {
    // Measured: only status 100 deletes; the message is the one sevDesk sent (body rebuilt by hand).
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(409).set_body_string(
            r#"{"error":{"message":"CheckAccountTransaction can only be deleted in status 100 but is status 400","code":159,"responseCode":409},"objects":null}"#,
        ))
        .mount(&server)
        .await;
    let err = client(&server)
        .delete_transaction(TransactionId::new(1))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            SevdeskError::Api {
                status: 409,
                code: Some(159),
                ..
            }
        ),
        "{err:?}"
    );
}
