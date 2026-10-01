//! The write answers recorded on the test account (2026-10-01), read through
//! the real client. Document ids and amounts are those of the recording.

use super::*;
use common::fixture;

async fn answering(
    verb: &str,
    route: &str,
    status: u16,
    fixture_name: &str,
) -> (MockServer, SevdeskClient) {
    let server = MockServer::start().await;
    Mock::given(method(verb))
        .and(path(route))
        .respond_with(ResponseTemplate::new(status).set_body_string(fixture(fixture_name)))
        .mount(&server)
        .await;
    let c = client(&server);
    (server, c)
}

fn save_invoice() -> SaveInvoice {
    SaveInvoice::new(common::invoice_draft(CurrencyCode::EUR), vec![])
}

#[tokio::test]
async fn the_recorded_save_invoice_answer_is_a_b2b_draft() {
    let (_s, c) = answering(
        "POST",
        "/Invoice/Factory/saveInvoice",
        200,
        "write_invoice_save",
    )
    .await;
    let invoice = c.save_invoice(&save_invoice()).await.unwrap().invoice;
    assert_eq!(invoice.status, Some(InvoiceStatus::Draft));
    assert_eq!(invoice.currency, CurrencyCode::EUR);
    assert_eq!(invoice.sum_net, Some(eur(5500)));
    assert_eq!(invoice.sum_gross, Some(eur(6545)));
    assert!(invoice.invoice_number.is_none());
    assert!(invoice.enshrined.is_none());
    assert!(invoice.update.is_some());
}

#[tokio::test]
async fn the_recorded_save_credit_note_answer_refers_to_its_invoice() {
    let (_s, c) = answering(
        "POST",
        "/CreditNote/Factory/saveCreditNote",
        200,
        "write_credit_note_save",
    )
    .await;
    let draft = common::credit_note_draft("GU-1048", CurrencyCode::EUR);
    let saved = c
        .save_credit_note(&SaveCreditNote::new(draft, vec![]))
        .await
        .unwrap()
        .credit_note;
    assert_eq!(saved.status, Some(CreditNoteStatus::Draft));
    assert_eq!(saved.credit_note_number.as_deref(), Some("GU-1048"));
    assert_eq!(saved.sum_gross, Some(eur(1190)));
    assert!(saved.ref_src_invoice.is_some());
    assert!(saved.origin.is_none());
}

#[tokio::test]
async fn the_recorded_cancellation_is_an_sr_pointing_to_its_original() {
    let (_s, c) = answering(
        "POST",
        "/Invoice/135582398/cancelInvoice",
        201,
        "write_invoice_cancel",
    )
    .await;
    let cancelled = c.cancel_invoice(InvoiceId::new(135582398)).await.unwrap();
    assert_eq!(cancelled.id, InvoiceId::new(135582436));
    assert!(cancelled.cancels(InvoiceId::new(135582398)));
    assert_eq!(cancelled.sum_gross, Some(eur(-2190)));
    assert_eq!(cancelled.currency, CurrencyCode::EUR);
}

#[tokio::test]
async fn the_recorded_pdf_and_xml_decode() {
    let (_s, c) = answering("GET", "/Invoice/5/getPdf", 200, "invoice_pdf").await;
    let pdf = c.invoice_pdf(InvoiceId::new(5)).await.unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF"));
    assert_eq!(pdf.filename, "RE-1076.pdf");
    assert_eq!(pdf.mime_type, "application/pdf");
    let (_s, c) = answering("GET", "/Invoice/5/getXml", 200, "invoice_xml").await;
    assert!(
        c.invoice_xml(InvoiceId::new(5))
            .await
            .unwrap()
            .starts_with("<?xml")
    );
}

#[tokio::test]
async fn the_recorded_contact_answers_decode() {
    let (_s, c) = answering("POST", "/Contact", 201, "write_contact_create").await;
    let new = NewContact::new(ContactName::Organisation { name: "x".into() });
    let contact = c.create_contact(&new).await.unwrap();
    assert_eq!(contact.buyer_reference.as_deref(), Some("-"));
    assert_eq!(
        contact.customer_number.as_deref(),
        Some("SDRS-LIVE-1790865321-contacts")
    );
    let (_s, c) = answering("GET", "/Contact", 200, "contact_list_by_customer_number").await;
    let found = c
        .find_contact_by_customer_number("SDRS-LIVE-1790865321-contacts")
        .await
        .unwrap();
    assert_eq!(found.unwrap().id, contact.id);
    let (_s, c) = answering("GET", "/ContactAddress", 200, "contact_address_list").await;
    let addresses = c.contact_addresses(contact.id).await.unwrap();
    assert_eq!(addresses.len(), 1);
    assert_eq!(
        addresses[0].category.map(|c| c.id),
        Some(AddressCategoryId::new(47))
    );
    let (_s, c) = answering("GET", "/CommunicationWay", 200, "communication_way_list").await;
    let ways = c.communication_ways(contact.id).await.unwrap();
    assert_eq!(ways[0].kind, Some(CommunicationWayType::Email));
}
