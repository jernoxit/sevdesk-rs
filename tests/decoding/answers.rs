//! Write answers recorded on the test account (2026-11-22), read through the
//! client: each is served on the route and with the method of its call.

use super::{client, eur, fixture};
use sevdesk::*;
use time::macros::datetime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn answering(verb: &str, route: &str, name: &str) -> (MockServer, SevdeskClient) {
    let server = MockServer::start().await;
    Mock::given(method(verb))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture(name)))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    (server, c)
}

fn booking(amount: i64) -> BookDocumentAmount {
    BookDocumentAmount {
        amount: eur(amount),
        date: datetime!(2026-11-05 12:00 +1),
        booking_type: BookingType::FullPayment,
        check_account: ObjectRef::new(CheckAccountId::new(1)),
        check_account_transaction: ObjectRef::new(TransactionId::new(2)),
    }
}

#[tokio::test]
async fn invoice_send_by_answers_the_open_invoice() {
    let (_s, c) = answering("PUT", "/Invoice/140050379/sendBy", "answer_invoice_sendBy").await;
    let i = c
        .send_invoice_by(InvoiceId::new(140050379), &SendBy::FINALIZE)
        .await
        .unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Open));
    assert_eq!(i.invoice_number.as_deref(), Some("RE-1305"));
    assert_eq!(i.sum_gross, Some(eur(1190)));
}

#[tokio::test]
async fn credit_note_send_by_answers_the_open_credit_note() {
    let (_s, c) = answering(
        "PUT",
        "/CreditNote/3117350/sendBy",
        "answer_credit_note_sendBy",
    )
    .await;
    let n = c
        .send_credit_note_by(CreditNoteId::new(3117350), &SendBy::FINALIZE)
        .await
        .unwrap();
    assert_eq!(n.status, Some(CreditNoteStatus::Open));
    assert_eq!(n.credit_note_number.as_deref(), Some("GU-1187"));
    assert_eq!(n.sum_gross, Some(eur(595)));
}

#[tokio::test]
async fn cancel_invoice_answers_the_cancellation_invoice() {
    let (_s, c) = answering(
        "POST",
        "/Invoice/140050379/cancelInvoice",
        "answer_cancelInvoice",
    )
    .await;
    let cancellation = c.cancel_invoice(InvoiceId::new(140050379)).await.unwrap();
    assert_eq!(cancellation.id, InvoiceId::new(140050397));
    assert!(cancellation.cancels(InvoiceId::new(140050379)));
    assert_eq!(cancellation.sum_gross, Some(eur(-1190)));
    assert_eq!(cancellation.status, Some(InvoiceStatus::Paid));
}

#[tokio::test]
async fn invoice_book_amount_answers_the_log_entry() {
    let (_s, c) = answering(
        "PUT",
        "/Invoice/140050379/bookAmount",
        "answer_invoice_bookAmount",
    )
    .await;
    let log = c
        .book_invoice_amount(InvoiceId::new(140050379), &booking(1190))
        .await
        .unwrap();
    assert_eq!(log.id, InvoiceLogId::new(227377056));
    assert_eq!(log.invoice, Some(ObjectRef::new(InvoiceId::new(140050379))));
    assert_eq!(log.from_status, Some(InvoiceStatus::Open));
    assert_eq!(log.to_status, Some(InvoiceStatus::Paid));
    // Spelled `ammountPayed` on the wire.
    assert_eq!(log.amount_payed, Some(eur(1190)));
    assert_eq!(log.booking_date, Some(datetime!(2026-11-05 12:00 +1)));
    assert_eq!(
        log.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert_eq!(log.create, Some(datetime!(2026-11-22 19:10:50 +1)));
}

#[tokio::test]
async fn credit_note_book_amount_answers_the_log_entry() {
    let (_s, c) = answering(
        "PUT",
        "/CreditNote/3117350/bookAmount",
        "answer_credit_note_bookAmount",
    )
    .await;
    let log = c
        .book_credit_note_amount(CreditNoteId::new(3117350), &booking(-595))
        .await
        .unwrap();
    assert_eq!(log.id, CreditNoteLogId::new(4264509));
    assert_eq!(
        log.credit_note,
        Some(ObjectRef::new(CreditNoteId::new(3117350)))
    );
    assert_eq!(log.from_status, Some(CreditNoteStatus::Open));
    assert_eq!(log.to_status, Some(CreditNoteStatus::Paid));
    assert_eq!(log.amount_payed, Some(eur(-595)));
}

#[tokio::test]
async fn reset_to_open_answers_the_document() {
    let (_s, c) = answering(
        "PUT",
        "/Invoice/140050379/resetToOpen",
        "answer_invoice_resetToOpen",
    )
    .await;
    let i = c
        .reset_invoice_to_open(InvoiceId::new(140050379))
        .await
        .unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Open));
    assert_eq!(i.paid_amount, None);
    let (_s, c) = answering(
        "PUT",
        "/CreditNote/3117350/resetToOpen",
        "answer_credit_note_resetToOpen",
    )
    .await;
    let n = c
        .reset_credit_note_to_open(CreditNoteId::new(3117350))
        .await
        .unwrap();
    assert_eq!(n.status, Some(CreditNoteStatus::Open));
}

#[tokio::test]
async fn reset_voucher_to_draft_answers_the_voucher() {
    let (_s, c) = answering(
        "PUT",
        "/Voucher/161453014/resetToDraft",
        "answer_voucher_resetToDraft",
    )
    .await;
    let v = c
        .reset_voucher_to_draft(VoucherId::new(161453014))
        .await
        .unwrap();
    assert_eq!(v.id, VoucherId::new(161453014));
    assert_eq!(v.status, Some(VoucherStatus::Draft));
}

fn english() -> DocumentParameter {
    DocumentParameter::Language(DocumentLanguage::EnUs)
}

#[tokio::test]
async fn invoice_change_parameter_answers_the_parameters() {
    let (_s, c) = answering(
        "PUT",
        "/Invoice/5/changeParameter",
        "answer_invoice_changeParameter",
    )
    .await;
    let p = c
        .change_invoice_parameter(InvoiceId::new(5), english())
        .await
        .unwrap();
    assert_eq!(p.result, None);
    let meta = p.meta_data.unwrap();
    assert_eq!(
        meta.doc_id.as_deref(),
        Some("fd0249ea79cfc8c81032b5e96c1acab2")
    );
    assert_eq!(meta.parameters.len(), 15);
    let language = &meta.parameters[0];
    assert_eq!(language.key, "language");
    assert_eq!(language.value.as_deref(), Some("en"));
    assert_eq!(language.visible, Some(true));
    assert_eq!(language.values.len(), 8);
    assert_eq!(
        language.values[0].translation_code.as_deref(),
        Some("SEVDOC_LANG_EN_US")
    );
    assert_eq!(language.values[0].is_custom, None);
    // The shortened fixture keeps the first 400 base64 characters: 300 bytes.
    let pdf = meta.pdf.unwrap();
    assert_eq!(pdf.len(), 300);
    assert!(pdf.starts_with(b"%PDF-"));
}

#[tokio::test]
async fn credit_note_change_parameter_answers_the_parameters_and_a_result() {
    let (_s, c) = answering(
        "PUT",
        "/CreditNote/6/changeParameter",
        "answer_credit_note_changeParameter",
    )
    .await;
    let p = c
        .change_credit_note_parameter(CreditNoteId::new(6), english())
        .await
        .unwrap();
    assert_eq!(p.result, Some(1));
    let meta = p.meta_data.unwrap();
    assert_eq!(meta.parameters[0].value.as_deref(), Some("en_US"));
    assert!(meta.pdf.unwrap().starts_with(b"%PDF-"));
}
