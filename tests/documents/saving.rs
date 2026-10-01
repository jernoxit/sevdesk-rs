//! Creating contacts, invoices and credit notes.

use super::*;

#[tokio::test]
async fn a_contact_is_created_as_a_customer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Contact"))
        .and(body_json(json!({
            "name": "SS-LIVE customer",
            "category": {"id": 3, "objectName": "Category"},
            "status": 1000
        })))
        .respond_with(answer(json!({"objects": {
            "id": "138682808", "objectName": "Contact", "name": "SS-LIVE customer"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let contact = client(&server)
        .create_contact(&NewContact::new(ContactName::Organisation {
            name: "SS-LIVE customer".into(),
        }))
        .await
        .unwrap();
    assert_eq!(contact.id, ContactId::new(138682808));
}

#[tokio::test]
async fn an_invoice_is_saved_as_a_draft_and_enshrined_by_send_by() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Invoice/Factory/saveInvoice"))
        .and(body_json(json!({
            "invoice": {
                "objectName": "Invoice", "mapAll": true,
                "contact": {"id": 7, "objectName": "Contact"},
                "contactPerson": {"id": 9, "objectName": "SevUser"},
                "invoiceDate": "29.09.2026", "deliveryDate": 1790632800,
                "header": "SS-LIVE-1", "discount": 0,
                "status": 100,
                "taxRule": {"id": 1, "objectName": "TaxRule"},
                "taxRate": 0, "taxText": "Umsatzsteuer 19%",
                "invoiceType": "RE", "currency": "EUR", "showNet": false,
                "customerInternalNote": "SS-LIVE-REF-1", "timeToPay": 14
            },
            "invoicePosSave": [{
                "objectName": "InvoicePos", "mapAll": true, "quantity": 1,
                "price": "10.00", "name": "Position",
                "unity": {"id": 1, "objectName": "Unity"},
                "taxRate": "19.00"
            }],
            "invoicePosDelete": null
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"objects": {
            "invoice": {
                "id": "135285186", "objectName": "Invoice", "invoiceNumber": null,
                "status": "100", "invoiceType": "RE", "currency": "EUR",
                "sumGross": "10", "paidAmount": 0,
                "customerInternalNote": "SS-LIVE-REF-1"
            },
            "invoicePos": []
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/Invoice/135285186/sendBy"))
        .and(body_json(json!({"sendType": "VPDF", "sendDraft": false})))
        .respond_with(answer(json!({"objects": {
            "id": "135285186", "objectName": "Invoice", "invoiceNumber": "RE-1024",
            "status": "200", "invoiceType": "RE", "currency": "EUR",
            "sumGross": "10", "paidAmount": 0
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    let draft = invoice_draft(CurrencyCode::EUR);
    let saved = c
        .save_invoice(&SaveInvoice::new(draft, vec![position("Position", 1000)]))
        .await
        .unwrap();
    assert_eq!(saved.invoice.status, Some(InvoiceStatus::Draft));
    assert_eq!(saved.invoice.invoice_number, None);
    assert_eq!(saved.invoice.sum_gross, Some(eur(1000)));
    let sent = c
        .send_invoice_by(saved.invoice.id, &SendBy::ENSHRINE)
        .await
        .unwrap();
    assert_eq!(sent.status, Some(InvoiceStatus::Open));
    assert_eq!(sent.invoice_number.as_deref(), Some("RE-1024"));
    assert_eq!(sent.currency, CurrencyCode::EUR);
}

#[tokio::test]
async fn a_credit_note_needs_its_number_and_the_provision_category() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/SevSequence/Factory/getByType"))
        .and(query_param("objectType", "CreditNote"))
        .and(query_param("type", "CN"))
        .respond_with(answer(json!({"objects": {
            "id": "13449786", "objectName": "SevSequence",
            "forObject": "CreditNote", "format": "GU-%NUMBER", "nextSequence": "1011", "type": "CN"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/CreditNote/Factory/saveCreditNote"))
        .and(body_json(json!({
            "creditNote": {
                "objectName": "CreditNote", "mapAll": true,
                "creditNoteNumber": "GU-1011",
                "contact": {"id": 7, "objectName": "Contact"},
                "contactPerson": {"id": 9, "objectName": "SevUser"},
                "creditNoteDate": "29.09.2026", "deliveryDate": 1790632800,
                "header": "SS-LIVE-2", "discount": 0,
                "status": 100,
                "taxRule": {"id": 9, "objectName": "TaxRule"},
                "taxRate": 0, "taxText": "Umsatzsteuer 19%",
                "creditNoteType": "CN", "bookingCategory": "PROVISION",
                "currency": "EUR", "showNet": false,
                "customerInternalNote": "SS-LIVE-REF-2"
            },
            "creditNotePosSave": [{
                "objectName": "CreditNotePos", "mapAll": true, "quantity": 1,
                "price": "4.00", "name": "Position",
                "unity": {"id": 1, "objectName": "Unity"},
                "taxRate": "19.00"
            }],
            "creditNotePosDelete": null
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"objects": {
            "creditNote": {
                "id": "2998960", "objectName": "CreditNote", "creditNoteNumber": "GU-1011",
                "status": "100", "currency": "EUR", "sumGross": "4"
            }
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/CreditNote/2998960/sendBy"))
        .and(body_json(json!({"sendType": "VPDF", "sendDraft": false})))
        .respond_with(answer(json!({"objects": {
            "id": "2998960", "objectName": "CreditNote", "creditNoteNumber": "GU-1011",
            "status": "200", "currency": "EUR", "sumGross": "4"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    let number = c.next_credit_note_number().await.unwrap();
    assert_eq!(number, "GU-1011");
    let draft = credit_note_draft(&number, CurrencyCode::EUR);
    let saved = c
        .save_credit_note(&SaveCreditNote::new(draft, vec![position("Position", 400)]))
        .await
        .unwrap();
    let sent = c
        .send_credit_note_by(saved.credit_note.id, &SendBy::ENSHRINE)
        .await
        .unwrap();
    assert_eq!(sent.status, Some(CreditNoteStatus::Open));
    assert_eq!(sent.credit_note_number.as_deref(), Some("GU-1011"));
}

#[tokio::test]
async fn a_sequence_without_the_number_placeholder_is_drift() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/SevSequence/Factory/getByType"))
        .respond_with(answer(json!({"objects": {
            "id": "1", "format": "GU-", "nextSequence": "1011"
        }})))
        .mount(&server)
        .await;
    let error = client(&server).next_credit_note_number().await.unwrap_err();
    assert!(matches!(error, SevdeskError::DriftGuard { .. }), "{error}");
}

#[tokio::test]
async fn sev_users_are_listed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/SevUser"))
        .respond_with(answer(json!({"objects": [
            {"id": "1551095", "objectName": "SevUser", "username": "api@example.invalid"}
        ]})))
        .mount(&server)
        .await;
    let users = client(&server).list_sev_users().await.unwrap();
    assert_eq!(users[0].id, SevUserId::new(1551095));
}
