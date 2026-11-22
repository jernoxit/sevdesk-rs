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
    assert_eq!(saved.sum_net, Some(eur(1000)));
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
    assert_eq!(
        cancelled.additional_information,
        Some(AdditionalInformation {
            credit_voucher: Some(false)
        })
    );
    assert_eq!(cancelled.property_use_new_doc_server, Some(true));
    assert_eq!(cancelled.property_is_e_invoice, None);
    assert_eq!(cancelled.send_type, Some(DocumentSendType::Vpdf));
    assert_eq!(cancelled.sum_tax, Some(eur(-350)));
    assert_eq!(
        cancelled.contact,
        Some(ObjectRef::new(ContactId::new(138810529)))
    );
    assert_eq!(cancelled.tax_rule, Some(ObjectRef::new(TaxRuleId::new(1))));
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

#[tokio::test]
async fn the_recorded_save_invoice_answer_carries_its_positions_with_numbers_and_true() {
    let (_s, c) = answering(
        "POST",
        "/Invoice/Factory/saveInvoice",
        200,
        "write_invoice_save",
    )
    .await;
    let saved = c.save_invoice(&save_invoice()).await.unwrap();
    assert_eq!(saved.vatdrop.as_deref(), Some("not relevant"));
    assert_eq!(saved.filename, None);
    let invoice = &saved.invoice;
    assert_eq!(invoice.send_type, Some(DocumentSendType::Unset));
    assert_eq!(invoice.property_is_e_invoice, Some(true));
    assert_eq!(invoice.sum_tax, Some(eur(1045)));
    assert_eq!(invoice.small_settlement, Some(false));
    assert_eq!(invoice.show_net, Some(true));
    assert_eq!(invoice.time_to_pay, Some(14));
    assert_eq!(
        invoice.contact,
        Some(ObjectRef::new(ContactId::new(138810483)))
    );
    assert_eq!(
        invoice.payment_method,
        Some(ObjectRef::new(PaymentMethodId::new(114251)))
    );
    assert_eq!(saved.invoice_pos.len(), 2);
    let (first, second) = (&saved.invoice_pos[0], &saved.invoice_pos[1]);
    assert_eq!(first.id, InvoicePositionId::new(362964135));
    assert_eq!(first.invoice, Some(ObjectRef::new(invoice.id)));
    assert_eq!(first.unity, Some(ObjectRef::new(UnityId::new(1))));
    assert_eq!(first.quantity, Some(Decimal::new(3, 0)));
    // The save answer writes a quantity of 1 as JSON `true`.
    assert_eq!(second.quantity, Some(Decimal::new(1, 0)));
    assert_eq!(first.price, Some(Decimal::new(10, 0)));
    assert_eq!(first.price_gross, Some(Decimal::new(119, 1)));
    assert_eq!(first.sum_gross, Some(eur(3570)));
    assert_eq!(first.tax_rate, Some(1900));
    assert_eq!(first.is_percentage, Some(true));
    assert_eq!(first.discount, Some(Decimal::new(0, 0)));
    assert_eq!(first.create_next_part, Some(false));
    assert_eq!(first.create, None);
    assert_eq!(first.temporary, None);
    assert_eq!(
        second.text.as_deref(),
        Some("Zweite Zeile\nund eine dritte")
    );
    assert_eq!(second.position_number, Some(1000));
}

#[tokio::test]
async fn the_quantity_probe_save_answer_reads_false_true_and_eight_digit_prices() {
    let (_s, c) = answering(
        "POST",
        "/Invoice/Factory/saveInvoice",
        200,
        "quantity_probe_save",
    )
    .await;
    let saved = c.save_invoice(&save_invoice()).await.unwrap();
    let quantities: Vec<_> = saved.invoice_pos.iter().map(|p| p.quantity).collect();
    assert_eq!(
        quantities,
        [
            Some(Decimal::new(0, 0)),
            Some(Decimal::new(5, 1)),
            Some(Decimal::new(1, 0)),
            Some(Decimal::new(2, 0)),
        ]
    );
    let half = &saved.invoice_pos[1];
    assert_eq!(half.price, Some(Decimal::new(123456, 4)));
    assert_eq!(half.price_gross, Some(Decimal::new(14691264, 6)));
    assert_eq!(half.price_tax, Some(Decimal::new(2345664, 6)));
    assert_eq!(half.sum_net, Some(eur(617)));
    assert_eq!(half.sum_gross, Some(eur(734)));
}

#[tokio::test]
async fn the_quantity_probe_get_invoice_pos_answer_reads_strings() {
    #[derive(serde::Deserialize)]
    struct Answer {
        objects: Vec<InvoicePosition>,
    }
    let answer: Answer =
        serde_json::from_str(&common::fixture("quantity_probe_get_invoice_pos")).unwrap();
    let positions = answer.objects;
    assert_eq!(positions.len(), 4);
    let half = &positions[1];
    assert_eq!(half.quantity, Some(Decimal::new(5, 1)));
    assert_eq!(half.price_gross, Some(Decimal::new(14691264, 6)));
    assert_eq!(half.price_tax, Some(Decimal::new(2345664, 6)));
    assert_eq!(half.temporary, Some(false));
    assert_eq!(half.is_percentage, Some(true));
    assert_eq!(half.position_number, Some(1000));
    assert_eq!(half.create, Some(datetime!(2026-11-22 13:14:21 +1)));
    assert_eq!(
        half.invoice,
        Some(ObjectRef::new(InvoiceId::new(140025486)))
    );
    assert_eq!(positions[0].quantity, Some(Decimal::new(0, 0)));
}
