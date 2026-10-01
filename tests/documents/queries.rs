//! Reading and filtering invoices, credit notes and the transaction log.

use super::*;

#[tokio::test]
async fn the_credit_note_filters_use_the_measured_spelling() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/CreditNote"))
        .and(query_param("customerInternalNote", "SS-LIVE-REF-2"))
        .and(query_param("creditNoteNumber", "GU-1011"))
        .respond_with(answer(json!({"objects": [
            {"id": "2998960", "creditNoteNumber": "GU-1011", "status": "200", "sumGross": "4",
             "currency": "EUR"}
        ]})))
        .expect(1)
        .mount(&server)
        .await;
    let found = client(&server)
        .list_credit_notes(&CreditNoteQuery {
            customer_internal_note: Some("SS-LIVE-REF-2".into()),
            credit_note_number: Some("GU-1011".into()),
            page: None,
        })
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
}

#[tokio::test]
async fn the_transaction_log_is_asked_by_the_transaction_reference() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/CheckAccountTransactionLog"))
        .and(query_param("checkAccountTransaction[id]", "77"))
        .and(query_param(
            "checkAccountTransaction[objectName]",
            "CheckAccountTransaction",
        ))
        .respond_with(answer(json!({"objects": []})))
        .expect(1)
        .mount(&server)
        .await;
    let log = client(&server)
        .transaction_log(TransactionId::new(77))
        .await
        .unwrap();
    assert!(log.is_empty());
}

#[tokio::test]
async fn a_log_page_is_asked_with_limit_and_offset_and_no_document_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/CheckAccountTransactionLog"))
        .and(query_param("limit", "25"))
        .and(query_param("offset", "50"))
        .respond_with(answer(json!({"objects": [
            {"id": "1", "objectName": "CheckAccountTransactionLog",
             "checkAccountTransaction": {"id": "7", "objectName": "CheckAccountTransaction"},
             "object": {"id": "2", "objectName": "CreditNote"}, "amountPaid": "-4"}
        ]})))
        .expect(1)
        .mount(&server)
        .await;
    let page = sevdesk::Page::new(25, 50).unwrap();
    let log = client(&server).transaction_log_page(page).await.unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(
        log[0].object,
        Some(BookedDocument::CreditNote(CreditNoteId::new(2)))
    );
}

#[tokio::test]
async fn a_cancellation_invoice_is_recognised_by_type_and_origin() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice"))
        .respond_with(answer(json!({"objects": [
            {"id": "1", "invoiceType": "RE", "status": "1000", "sumGross": "10", "currency": "EUR"},
            {"id": "2", "invoiceType": "SR", "status": "1000", "sumGross": "-10",
             "currency": "EUR", "origin": {"id": "1", "objectName": "Invoice"}},
            {"id": "3", "invoiceType": "SR", "status": "1000", "sumGross": "-10",
             "currency": "EUR", "origin": {"id": "99", "objectName": "Invoice"}}
        ]})))
        .mount(&server)
        .await;
    let all = client(&server)
        .list_invoices(&InvoiceQuery::default())
        .await
        .unwrap();
    let original = InvoiceId::new(1);
    let cancelling: Vec<_> = all.iter().filter(|i| i.cancels(original)).collect();
    assert_eq!(cancelling.len(), 1);
    assert_eq!(cancelling[0].id, InvoiceId::new(2));
}

#[tokio::test]
async fn update_after_goes_out_as_unix_seconds_and_a_consistent_answer_passes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice"))
        .and(query_param("updateAfter", "1790632800"))
        .respond_with(answer(json!({"objects": [
            {"id": "1", "status": "200", "currency": "EUR",
             "update": "2026-09-29T00:00:00+02:00"},
            {"id": "2", "status": "200", "currency": "EUR",
             "update": "2026-09-30T10:00:00+02:00"}
        ]})))
        .expect(1)
        .mount(&server)
        .await;
    let query = InvoiceQuery {
        update_after: Some(datetime!(2026-09-29 0:00 +2)),
        ..Default::default()
    };
    assert_eq!(
        client(&server).list_invoices(&query).await.unwrap().len(),
        2
    );
}

#[tokio::test]
async fn an_element_older_than_update_after_is_a_drift_guard() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice"))
        .respond_with(answer(json!({"objects": [
            {"id": "1", "status": "200", "currency": "EUR",
             "update": "2026-09-29T00:00:00+02:00"},
            {"id": "2", "status": "200", "currency": "EUR",
             "update": "2026-09-01T10:00:00+02:00"}
        ]})))
        .mount(&server)
        .await;
    let query = InvoiceQuery {
        update_after: Some(datetime!(2026-09-29 0:00 +2)),
        ..Default::default()
    };
    let err = client(&server).list_invoices(&query).await.unwrap_err();
    assert!(
        matches!(
            err,
            SevdeskError::DriftGuard {
                context: "GET /Invoice updateAfter",
                ..
            }
        ),
        "{err:?}"
    );
}
