//! The shape of the draft bodies and the document parameter.

use super::*;
use time::macros::date;

#[allow(
    clippy::unwrap_used,
    reason = "test helper outside #[test] functions: a failure here is a test failure"
)]
/// The JSON body `save` posts to `route`, read back from the mock server.
async fn posted_body(route: &str, post: impl AsyncFnOnce(&SevdeskClient)) -> serde_json::Value {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"objects": {
            "invoice": {"id": "1", "status": "100", "currency": "EUR", "sumGross": "1"},
            "creditNote": {"id": "1", "status": "100", "currency": "EUR", "sumGross": "1"}
        }})))
        .expect(1)
        .mount(&server)
        .await;
    post(&client(&server)).await;
    let requests: Vec<Request> = server.received_requests().await.unwrap();
    serde_json::from_slice(&requests[0].body).unwrap()
}

#[tokio::test]
async fn a_b2b_invoice_draft_sends_address_payment_method_period_and_e_invoice() {
    let mut draft = invoice_draft(CurrencyCode::EUR);
    draft.show_net = true;
    draft.delivery_date = datetime!(2026-09-01 0:00 +2);
    draft.options.delivery_date_until = Some(datetime!(2026-09-30 0:00 +2));
    draft.options.head_text = Some("Head".into());
    draft.options.foot_text = Some("Foot".into());
    draft.options.address_name = Some("Probe GmbH".into());
    draft.options.address_street = Some("Probestrasse 1".into());
    draft.options.address_zip = Some("10115".into());
    draft.options.address_city = Some("Berlin".into());
    draft.options.address_country = Some(ObjectRef::new(CountryId::new(1)));
    draft.options.payment_method = Some(ObjectRef::new(PaymentMethodId::new(21919)));
    draft.options.is_e_invoice = true;
    let mut discount = position("Rabatt", 1000);
    discount.quantity = -1;
    discount.options.text = Some("Aktion".into());
    let save = SaveInvoice::new(draft, vec![position("Abo", 4900), discount]);
    let body = posted_body("/Invoice/Factory/saveInvoice", async |c| {
        c.save_invoice(&save).await.unwrap();
    })
    .await;
    assert_eq!(
        body,
        json!({
            "invoice": {
                "objectName": "Invoice", "mapAll": true,
                "contact": {"id": 7, "objectName": "Contact"},
                "contactPerson": {"id": 9, "objectName": "SevUser"},
                "invoiceDate": "29.09.2026",
                "deliveryDate": 1788213600, "deliveryDateUntil": 1790719200,
                "header": "SS-LIVE-1", "headText": "Head", "footText": "Foot",
                "discount": 0, "status": 100,
                "taxRule": {"id": 1, "objectName": "TaxRule"},
                "taxRate": 0, "taxText": "Umsatzsteuer 19%",
                "invoiceType": "RE", "currency": "EUR", "showNet": true,
                "timeToPay": 14, "customerInternalNote": "SS-LIVE-REF-1",
                "addressName": "Probe GmbH", "addressStreet": "Probestrasse 1",
                "addressZip": "10115", "addressCity": "Berlin",
                "addressCountry": {"id": 1, "objectName": "StaticCountry"},
                "paymentMethod": {"id": 21919, "objectName": "PaymentMethod"},
                "propertyIsEInvoice": true
            },
            "invoicePosSave": [
                {"objectName": "InvoicePos", "mapAll": true, "quantity": 1,
                 "price": "49.00", "name": "Abo",
                 "unity": {"id": 1, "objectName": "Unity"}, "taxRate": "19.00"},
                {"objectName": "InvoicePos", "mapAll": true, "quantity": -1,
                 "price": "10.00", "name": "Rabatt", "text": "Aktion",
                 "unity": {"id": 1, "objectName": "Unity"}, "taxRate": "19.00"}
            ],
            "invoicePosDelete": null
        })
    );
}

#[tokio::test]
async fn a_b2c_invoice_draft_leaves_every_absent_optional_out() {
    let save = SaveInvoice::new(
        invoice_draft(CurrencyCode::EUR),
        vec![position("Position", 1000)],
    );
    let body = posted_body("/Invoice/Factory/saveInvoice", async |c| {
        c.save_invoice(&save).await.unwrap();
    })
    .await;
    let invoice = body["invoice"].as_object().unwrap();
    for absent in [
        "deliveryDateUntil",
        "headText",
        "footText",
        "addressName",
        "addressStreet",
        "addressZip",
        "addressCity",
        "addressCountry",
        "paymentMethod",
        "propertyIsEInvoice",
    ] {
        assert!(!invoice.contains_key(absent), "{absent} must be absent");
    }
    assert_eq!(invoice["showNet"], json!(false));
    let pos = body["invoicePosSave"][0].as_object().unwrap();
    assert!(!pos.contains_key("text"));
    assert!(!pos.contains_key("positionNumber"));
}

#[tokio::test]
async fn a_credit_note_draft_carries_the_invoice_reference() {
    let mut draft = credit_note_draft("GU-1000", CurrencyCode::EUR);
    draft.booking_category = CreditNoteCategory::Underachievement;
    draft.options.ref_src_invoice = Some(ObjectRef::new(InvoiceId::new(135169190)));
    draft.tax_rule = ObjectRef::new(TaxRuleId::new(1));
    draft.show_net = true;
    draft.options.delivery_date_until = Some(datetime!(2026-09-30 0:00 +2));
    let save = SaveCreditNote::new(draft, vec![position("Teilgutschrift", 500)]);
    let body = posted_body("/CreditNote/Factory/saveCreditNote", async |c| {
        c.save_credit_note(&save).await.unwrap();
    })
    .await;
    assert_eq!(
        body,
        json!({
            "creditNote": {
                "objectName": "CreditNote", "mapAll": true,
                "creditNoteNumber": "GU-1000",
                "contact": {"id": 7, "objectName": "Contact"},
                "contactPerson": {"id": 9, "objectName": "SevUser"},
                "creditNoteDate": "29.09.2026",
                "deliveryDate": 1790632800, "deliveryDateUntil": 1790719200,
                "header": "SS-LIVE-2", "discount": 0, "status": 100,
                "taxRule": {"id": 1, "objectName": "TaxRule"},
                "taxRate": 0, "taxText": "Umsatzsteuer 19%",
                "creditNoteType": "CN", "bookingCategory": "UNDERACHIEVEMENT",
                "refSrcInvoice": {"id": 135169190, "objectName": "Invoice"},
                "currency": "EUR", "showNet": true,
                "customerInternalNote": "SS-LIVE-REF-2"
            },
            "creditNotePosSave": [
                {"objectName": "CreditNotePos", "mapAll": true, "quantity": 1,
                 "price": "5.00", "name": "Teilgutschrift",
                 "unity": {"id": 1, "objectName": "Unity"}, "taxRate": "19.00"}
            ],
            "creditNotePosDelete": null
        })
    );
}

#[tokio::test]
async fn a_credit_note_draft_without_reference_leaves_it_out() {
    let save = SaveCreditNote::new(
        credit_note_draft("GU-1", CurrencyCode::EUR),
        vec![position("Position", 400)],
    );
    let body = posted_body("/CreditNote/Factory/saveCreditNote", async |c| {
        c.save_credit_note(&save).await.unwrap();
    })
    .await;
    assert_eq!(body["creditNote"]["bookingCategory"], json!("PROVISION"));
    assert!(body["creditNote"].get("refSrcInvoice").is_none());
    assert!(body["creditNote"].get("propertyIsEInvoice").is_none());
    assert!(body["creditNote"].get("paymentMethod").is_none());
}

#[tokio::test]
async fn the_document_language_is_changed_with_a_typed_parameter() {
    let server = MockServer::start().await;
    for document in ["Invoice/5", "CreditNote/6"] {
        Mock::given(method("PUT"))
            .and(path(format!("/{document}/changeParameter")))
            .and(body_json(json!({"key": "language", "value": "en_US"})))
            .respond_with(answer(json!({"objects": {"metaData": {"parameters": []}}})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);
    let english = DocumentParameter::Language(DocumentLanguage::EnUs);
    c.change_invoice_parameter(InvoiceId::new(5), english)
        .await
        .unwrap();
    c.change_credit_note_parameter(CreditNoteId::new(6), english)
        .await
        .unwrap();
}

#[tokio::test]
async fn an_e_invoice_credit_note_draft_sends_the_flag() {
    let mut draft = credit_note_draft("GU-1", CurrencyCode::EUR);
    draft.options.is_e_invoice = true;
    draft.options.payment_method = Some(ObjectRef::new(PaymentMethodId::new(21919)));
    let save = SaveCreditNote::new(draft, vec![position("Position", 500)]);
    let body = posted_body("/CreditNote/Factory/saveCreditNote", async |c| {
        c.save_credit_note(&save).await.unwrap();
    })
    .await;
    assert_eq!(body["creditNote"]["propertyIsEInvoice"], json!(true));
    assert_eq!(
        body["creditNote"]["paymentMethod"],
        json!({"id": 21919, "objectName": "PaymentMethod"})
    );
}

#[test]
fn every_measured_invoice_option_serializes_with_its_measured_encoding() {
    let mut draft = invoice_draft(CurrencyCode::EUR);
    draft.options.address = Some("SDRS Probe\nProbestrasse 9\n10115 Berlin\nGermany".into());
    draft.options.small_settlement = Some(true);
    draft.options.pay_date = Some(date!(2026 - 11 - 20));
    #[allow(
        deprecated,
        reason = "the test pins the encoding of the deprecated options"
    )]
    {
        draft.options.send_date = Some(date!(2026 - 11 - 20));
        draft.options.send_type = Some(SendType::Vpdf);
        draft.options.invoice_number = Some("SDRS-FLD-1-NR".into());
    }
    let mut item = position("Abo", 4900);
    item.options.position_number = Some(3);
    let body = serde_json::to_value(SaveInvoice::new(draft, vec![item])).unwrap();
    let invoice = &body["invoice"];
    assert_eq!(
        invoice["address"],
        json!("SDRS Probe\nProbestrasse 9\n10115 Berlin\nGermany")
    );
    assert_eq!(invoice["smallSettlement"], json!(true));
    assert_eq!(invoice["sendDate"], json!("20.11.2026"));
    assert_eq!(invoice["payDate"], json!("20.11.2026"));
    assert_eq!(invoice["sendType"], json!("VPDF"));
    assert_eq!(invoice["invoiceNumber"], json!("SDRS-FLD-1-NR"));
    assert_eq!(body["invoicePosSave"][0]["positionNumber"], json!(3));
}

#[test]
fn every_measured_credit_note_option_serializes_with_its_measured_encoding() {
    let mut draft = credit_note_draft("GU-1", CurrencyCode::EUR);
    draft.options.address = Some("SDRS Probe\nProbestrasse 9".into());
    draft.options.small_settlement = Some(true);
    #[allow(
        deprecated,
        reason = "the test pins the encoding of the deprecated options"
    )]
    {
        draft.options.send_date = Some(date!(2026 - 11 - 20));
        draft.options.send_type = Some(SendType::Vpdf);
    }
    let mut item = position("Gutschrift", 500);
    item.options.position_number = Some(3);
    let body = serde_json::to_value(SaveCreditNote::new(draft, vec![item])).unwrap();
    let note = &body["creditNote"];
    assert_eq!(note["address"], json!("SDRS Probe\nProbestrasse 9"));
    assert_eq!(note["smallSettlement"], json!(true));
    assert_eq!(note["sendDate"], json!("20.11.2026"));
    assert_eq!(note["sendType"], json!("VPDF"));
    assert_eq!(body["creditNotePosSave"][0]["positionNumber"], json!(3));
}

#[test]
fn an_untouched_options_struct_adds_none_of_the_new_keys() {
    let body = serde_json::to_value(SaveInvoice::new(
        invoice_draft(CurrencyCode::EUR),
        vec![position("Abo", 4900)],
    ))
    .unwrap();
    let invoice = body["invoice"].as_object().unwrap();
    for absent in [
        "address",
        "smallSettlement",
        "sendDate",
        "payDate",
        "sendType",
        "invoiceNumber",
    ] {
        assert!(!invoice.contains_key(absent), "{absent} must be absent");
    }
}

#[test]
fn every_measured_voucher_option_serializes_with_its_measured_encoding() {
    let mut voucher = VoucherDraft::new(
        date!(2026 - 11 - 20),
        "Supplier",
        "R-1",
        TaxRuleId::new(14),
        CurrencyCode::EUR,
    );
    voucher.options.payment_deadline = Some(date!(2026 - 12 - 01));
    voucher.options.document = Some(ObjectRef::new(DocumentId::new(310507791)));
    voucher.options.pay_date = Some(date!(2026 - 11 - 20));
    let mut gross = VoucherPositionDraft::new(AccountDatevId::new(4285), 1900, eur(35), "gross");
    gross.options.accounting_type = Some(ObjectRef::new(AccountingTypeId::new(32890)));
    gross.options.is_asset = Some(true);
    gross.options.sum_gross = Some(eur(42));
    let net = VoucherPositionDraft::new(AccountDatevId::new(4285), 1900, eur(35), "net");
    let body = serde_json::to_value(SaveVoucher {
        voucher,
        positions: vec![gross, net],
        filename: None,
    })
    .unwrap();
    let v = &body["voucher"];
    assert_eq!(v["paymentDeadline"], json!("01.12.2026"));
    assert_eq!(
        v["document"],
        json!({"id": 310507791, "objectName": "Document"})
    );
    assert_eq!(v["payDate"], json!("20.11.2026"));
    let p = &body["voucherPosSave"][0];
    assert_eq!(
        p["accountingType"],
        json!({"id": 32890, "objectName": "AccountingType"})
    );
    assert_eq!(p["isAsset"], json!(true));
    assert_eq!(p["sumGross"], json!("0.42"));
    assert_eq!(p["net"], json!(false));
    let n = body["voucherPosSave"][1].as_object().unwrap();
    assert_eq!(n["net"], json!(true));
    assert!(!n.contains_key("sumGross"));
}
