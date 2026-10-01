//! The shape of the draft bodies and the document parameter.

use super::*;

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
    draft.delivery_date_until = Some(datetime!(2026-09-30 0:00 +2));
    draft.head_text = Some("Head".into());
    draft.foot_text = Some("Foot".into());
    draft.address_name = Some("Probe GmbH".into());
    draft.address_street = Some("Probestrasse 1".into());
    draft.address_zip = Some("10115".into());
    draft.address_city = Some("Berlin".into());
    draft.address_country = Some(ObjectRef::new(CountryId::new(1)));
    draft.payment_method = Some(ObjectRef::new(PaymentMethodId::new(21919)));
    draft.is_e_invoice = true;
    let mut discount = position("Rabatt", 1000);
    discount.quantity = -1;
    discount.text = Some("Aktion".into());
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
    draft.ref_src_invoice = Some(ObjectRef::new(InvoiceId::new(135169190)));
    draft.tax_rule = ObjectRef::new(TaxRuleId::new(1));
    draft.show_net = true;
    draft.delivery_date_until = Some(datetime!(2026-09-30 0:00 +2));
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
    draft.is_e_invoice = true;
    draft.payment_method = Some(ObjectRef::new(PaymentMethodId::new(21919)));
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
