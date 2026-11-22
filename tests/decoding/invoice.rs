//! Fixture decoding of invoices.

use super::{assert_probe_address, client, eur, fixture, sample_positions, serving};
use sevdesk::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn invoice_list_with_cancellation_pointing_to_origin() {
    let (_s, c) = serving("/Invoice", "invoice_list").await;
    let list = c.list_invoices(&InvoiceQuery::default()).await.unwrap();
    let sr = list
        .iter()
        .find(|i| i.invoice_type == Some(InvoiceType::Cancellation))
        .unwrap();
    assert_eq!(sr.invoice_number.as_deref(), Some("RE-1018"));
    assert_eq!(sr.origin.unwrap().id, InvoiceId::new(135171691));
    assert_eq!(sr.sum_gross, Some(eur(-4665)));
    assert_eq!(sr.currency, CurrencyCode::EUR);
    assert_eq!(sr.status, Some(InvoiceStatus::Paid));
    let re = list
        .iter()
        .find(|i| i.id == InvoiceId::new(135171691))
        .unwrap();
    assert_eq!(re.invoice_type, Some(InvoiceType::Invoice));
    assert!(re.origin.is_none());
    assert_eq!(re.customer_internal_note, sr.customer_internal_note);
    assert_probe_address(
        re.address_name.as_deref(),
        re.address_street.as_deref(),
        re.address_zip.as_deref(),
        re.address_city.as_deref(),
        re.address_country,
    );
}

#[tokio::test]
async fn invoice_draft_has_update_and_no_enshrined() {
    let (_s, c) = serving("/Invoice/135169163", "invoice_draft").await;
    let i = c.invoice(InvoiceId::new(135169163)).await.unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Draft));
    assert_eq!(i.invoice_number, None);
    assert_eq!(
        i.update,
        Some(time::macros::datetime!(2026-09-28 19:39:15 +2))
    );
    assert_eq!(i.enshrined, None);
    assert_eq!(i.sum_net, Some(eur(2900)));
    assert_eq!(i.sum_gross, Some(eur(3451)));
    assert_eq!(
        i.delivery_date,
        Some(time::macros::datetime!(2026-09-01 0:00 +2))
    );
    assert_eq!(
        i.delivery_date_until,
        Some(time::macros::datetime!(2026-09-30 0:00 +2))
    );
}

#[tokio::test]
async fn enshrined_invoice_has_number_update_and_enshrined() {
    let (_s, c) = serving("/Invoice/135169163", "invoice_enshrined").await;
    let i = c.invoice(InvoiceId::new(135169163)).await.unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Open));
    assert_eq!(i.invoice_number.as_deref(), Some("RE-1000"));
    assert_eq!(
        i.update,
        Some(time::macros::datetime!(2026-09-28 19:39:19 +2))
    );
    assert_eq!(
        i.enshrined,
        Some(time::macros::datetime!(2026-09-28 19:39:19 +2))
    );
    assert_eq!(i.currency, CurrencyCode::EUR);
    assert_eq!(i.send_type, Some(DocumentSendType::Vpdf));
    assert_eq!(
        i.send_date,
        Some(time::macros::datetime!(2026-09-28 19:39:18 +2))
    );
    assert_eq!(i.header.as_deref(), Some("Rechnung PROBE-R1 A1-str-date"));
}

#[tokio::test]
async fn invoice_draft_reads_every_other_key() {
    let (_s, c) = serving("/Invoice/135169163", "invoice_draft").await;
    let i = c.invoice(InvoiceId::new(135169163)).await.unwrap();
    assert_eq!(i.send_type, Some(DocumentSendType::Unset));
    assert_eq!(i.small_settlement, Some(false));
    assert_eq!(i.show_net, Some(true));
    assert_eq!(i.time_to_pay, Some(14));
    assert_eq!(i.discount, Some(0));
    assert_eq!(i.tax_rate, Some(0));
    assert_eq!(i.sum_tax, Some(eur(551)));
    assert_eq!(i.paid_amount, Some(eur(0)));
    assert_eq!(i.reminder_charge, Some(eur(0)));
    assert_eq!(i.reminder_total, None);
    assert_eq!(i.tax_type, None);
    assert_eq!(i.pay_date, None);
    assert_eq!(i.contact, Some(ObjectRef::new(ContactId::new(138615465))));
    assert_eq!(
        i.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert_eq!(i.create_user, Some(ObjectRef::new(SevUserId::new(1551095))));
    assert_eq!(i.tax_rule, Some(ObjectRef::new(TaxRuleId::new(1))));
    assert_eq!(
        i.create,
        Some(time::macros::datetime!(2026-09-28 19:39:15 +2))
    );
    assert_eq!(i.tax_text.as_deref(), Some("Umsatzsteuer 19%"));
}

#[test]
fn invoice_keys_in_unmeasured_forms_are_decode_errors_naming_the_key() {
    let read = |extra: &str| {
        serde_json::from_str::<Invoice>(&format!(r#"{{"id": "1", "currency": "EUR", {extra}}}"#))
    };
    assert!(read(r#""showNet": "2""#).is_err());
    assert!(read(r#""sumTax": "1.234""#).is_err());
    assert_eq!(
        read(r#""sendType": "ZZ""#).unwrap().send_type,
        Some(DocumentSendType::Other("ZZ".into()))
    );
    assert_eq!(
        read(r#""invoiceType": "WKR""#).unwrap().invoice_type,
        Some(InvoiceType::Recurring)
    );
    assert_eq!(
        read(r#""taxType": "noteu""#).unwrap().tax_type,
        Some(TaxType::NotEu)
    );
}

#[tokio::test]
async fn invoice_draft_has_null_address_strings_and_a_country() {
    let (_s, c) = serving("/Invoice/135169163", "invoice_draft").await;
    let i = c.invoice(InvoiceId::new(135169163)).await.unwrap();
    assert_eq!(i.address_name, None);
    assert_eq!(i.address_street, None);
    assert_eq!(i.address_zip, None);
    assert_eq!(i.address_city, None);
    assert_eq!(i.address_country, Some(ObjectRef::new(CountryId::new(1))));
}

#[test]
fn address_strings_read_empty_null_and_absent_as_none() {
    let read = |extra: &str| -> Invoice {
        serde_json::from_str(&format!(r#"{{"id": "1", "currency": "EUR"{extra}}}"#)).unwrap()
    };
    assert_eq!(read(r#", "addressName": """#).address_name, None);
    assert_eq!(read(r#", "addressName": null"#).address_name, None);
    assert_eq!(read("").address_name, None);
    assert_eq!(
        read(r#", "addressName": "x""#).address_name.as_deref(),
        Some("x")
    );
}

#[tokio::test]
async fn decode_error_names_the_field_path_and_the_status() {
    let mut body: serde_json::Value = serde_json::from_str(&fixture("invoice_list")).unwrap();
    body["objects"][0]["sumNet"] = serde_json::json!("1.234");
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body.to_string()))
        .mount(&server)
        .await;
    let err = client(&server)
        .list_invoices(&InvoiceQuery::default())
        .await
        .unwrap_err();
    match &err {
        SevdeskError::Decode {
            status,
            path,
            context,
            ..
        } => {
            assert_eq!(
                (*status, path.as_str(), *context),
                (200, "[0].sumNet", "Invoice")
            );
        }
        other => panic!("expected a decode error, got {other:?}"),
    }
    let message = err.to_string();
    assert!(
        message.contains("HTTP 200") && message.contains("at `[0].sumNet`"),
        "{message}"
    );
}

#[tokio::test]
async fn sample_invoice_list_reads_every_measured_form() {
    let (_s, c) = serving("/Invoice", "sample_invoice_list").await;
    let list = c.list_invoices(&InvoiceQuery::default()).await.unwrap();
    assert_eq!(list.len(), 4);
    let by_id = |id: i64| list.iter().find(|i| i.id == InvoiceId::new(id)).unwrap();

    let open = by_id(135171669);
    assert_eq!(open.invoice_type, Some(InvoiceType::Invoice));
    assert_eq!(open.status, Some(InvoiceStatus::Open));
    assert_eq!(open.sum_net, Some(eur(3900)));
    assert_eq!(open.sum_tax, Some(eur(741)));
    assert_eq!(open.sum_gross, Some(eur(4641)));
    assert_eq!(open.show_net, Some(true));
    assert_eq!(open.small_settlement, Some(false));
    assert_eq!(open.send_type, Some(DocumentSendType::Vpdf));
    assert_eq!(open.reminder_charge, Some(eur(0)));
    assert_eq!(
        open.enshrined,
        Some(time::macros::datetime!(2026-09-28 20:15:44 +2))
    );
    assert_eq!(
        open.delivery_date_until,
        Some(time::macros::datetime!(2026-10-27 0:00:00 +1))
    );

    let draft = by_id(135173157);
    assert_eq!(draft.status, Some(InvoiceStatus::Draft));
    assert_eq!(draft.enshrined, None);
    assert_eq!(draft.send_type, Some(DocumentSendType::Unset));
    assert_eq!(draft.einvoice_reference.as_deref(), Some("CUSTOM"));
    assert_eq!(draft.sum_gross, Some(eur(0)));
    assert!(draft.contact.is_none());

    let cancellation = by_id(135582249);
    assert_eq!(cancellation.invoice_type, Some(InvoiceType::Cancellation));
    assert_eq!(cancellation.origin.unwrap().id, InvoiceId::new(135582230));
    assert_eq!(cancellation.sum_net, Some(eur(-1840)));
    assert_eq!(cancellation.sum_tax, Some(eur(-350)));
    assert_eq!(cancellation.sum_gross, Some(eur(-2190)));
    assert_eq!(cancellation.show_net, Some(false));
    assert_eq!(cancellation.status, Some(InvoiceStatus::Paid));

    let paid = by_id(135169196);
    assert_eq!(paid.status, Some(InvoiceStatus::Paid));
    assert_eq!(
        paid.pay_date,
        Some(time::macros::datetime!(2026-09-28 0:00:00 +2))
    );
    assert_eq!(
        paid.delivery_date,
        Some(time::macros::datetime!(2026-09-01 0:00:01 +2))
    );
}

#[test]
fn sample_invoice_pos_list_reads_every_measured_form() {
    let list = sample_positions::<InvoicePosition>("sample_invoice_pos_list");
    assert_eq!(list.len(), 4);
    let by_id = |id: i64| {
        list.iter()
            .find(|p| p.id == InvoicePositionId::new(id))
            .unwrap()
    };

    let discounted = by_id(361892829);
    assert_eq!(discounted.quantity, Some(Decimal::new(1, 0)));
    assert_eq!(discounted.discount, Some(Decimal::new(10, 0)));
    assert_eq!(discounted.discounted_value, Some(Decimal::new(10, 0)));
    assert_eq!(discounted.is_percentage, Some(true));
    assert_eq!(discounted.price_gross, Some(Decimal::new(595, 1)));
    assert_eq!(discounted.sum_discount, Some(eur(500)));
    assert_eq!(discounted.sum_gross, Some(eur(5355)));
    assert_eq!(discounted.temporary, Some(false));
    assert_eq!(discounted.tax_rate, Some(1900));
    assert_eq!(discounted.position_number, Some(1000));
    assert_eq!(
        discounted.invoice,
        Some(ObjectRef::new(InvoiceId::new(135169899)))
    );
    assert_eq!(discounted.unity, Some(ObjectRef::new(UnityId::new(1))));
    assert_eq!(
        discounted.create,
        Some(time::macros::datetime!(2026-09-28 19:49:37 +2))
    );

    let empty = by_id(361902006);
    assert_eq!(empty.name, None); // an empty string reads as None
    assert_eq!(empty.price, Some(Decimal::new(0, 0)));

    let gross = by_id(361892841);
    assert_eq!(gross.quantity, Some(Decimal::new(5, 1)));
    assert_eq!(gross.price, Some(Decimal::new(777, 2)));
    assert_eq!(gross.price_net, Some(Decimal::new(652941176, 8)));
    assert_eq!(gross.price_tax, Some(Decimal::new(124058824, 8)));
    assert_eq!(gross.sum_net, Some(eur(327)));
    assert_eq!(gross.sum_gross, Some(eur(389)));
}
