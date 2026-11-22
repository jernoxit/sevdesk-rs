//! Fixture decoding of credit notes.

use super::{assert_probe_address, eur, sample_positions, serving};
use sevdesk::*;

#[test]
fn credit_note_without_sum_net_reads_none() {
    let n: CreditNote = serde_json::from_str(r#"{"id": "1", "currency": "EUR"}"#).unwrap();
    assert_eq!(n.sum_net, None);
}

#[tokio::test]
async fn credit_note() {
    let (_s, c) = serving("/CreditNote/2996041", "credit_note").await;
    let n = c.credit_note(CreditNoteId::new(2996041)).await.unwrap();
    assert_eq!(n.credit_note_number.as_deref(), Some("GU-1006"));
    assert_eq!(n.sum_net, Some(eur(4900)));
    assert_eq!(n.sum_gross, Some(eur(5831)));
    assert_eq!(n.currency, CurrencyCode::EUR);
    assert_eq!(n.status, Some(CreditNoteStatus::Paid));
    assert_eq!(n.origin.unwrap().id, InvoiceId::new(135171965));
    assert_probe_address(
        n.address_name.as_deref(),
        n.address_street.as_deref(),
        n.address_zip.as_deref(),
        n.address_city.as_deref(),
        n.address_country,
    );
}

#[tokio::test]
async fn credit_note_reads_every_other_key() {
    let (_s, c) = serving("/CreditNote/2996041", "credit_note").await;
    let n = c.credit_note(CreditNoteId::new(2996041)).await.unwrap();
    assert_eq!(n.send_type, Some(DocumentSendType::Vpdf));
    assert_eq!(
        n.booking_category,
        Some(CreditNoteBookingCategory::Underachievement)
    );
    assert_eq!(n.credit_note_type.as_deref(), Some("CN"));
    assert_eq!(n.sum_tax, Some(eur(931)));
    assert_eq!(n.sum_net_accounting, Some(eur(4900)));
    assert_eq!(n.sum_discounts, Some(eur(0)));
    assert_eq!(n.show_net, Some(true));
    assert_eq!(n.small_settlement, Some(false));
    assert_eq!(n.is_transferred, Some(false));
    assert_eq!(n.discount_time, Some(0));
    assert_eq!(n.discount, Some(0));
    assert_eq!(n.tax_rate, Some(0));
    assert_eq!(n.tax_type, None);
    assert_eq!(n.paid_amount, None);
    assert_eq!(n.head_text, None);
    assert_eq!(
        n.header.as_deref(),
        Some("Gutschrift Nr. GU-1006 zur Rechnung Nr. RE-1020")
    );
    assert_eq!(
        n.create,
        Some(time::macros::datetime!(2026-09-29 1:30:50 +2))
    );
    assert_eq!(
        n.pay_date,
        Some(time::macros::datetime!(2026-08-26 12:00:00 +2))
    );
    assert_eq!(
        n.delivery_date_until,
        Some(time::macros::datetime!(2026-10-27 0:00:00 +1))
    );
    assert_eq!(n.contact, Some(ObjectRef::new(ContactId::new(138615465))));
    assert_eq!(
        n.ref_src_invoice,
        Some(ObjectRef::new(InvoiceId::new(135171965)))
    );
    assert_eq!(n.tax_rule, Some(ObjectRef::new(TaxRuleId::new(1))));
    assert_eq!(n.property_is_e_invoice, None);
}

#[test]
fn credit_note_keys_in_unmeasured_forms() {
    let read = |extra: &str| {
        serde_json::from_str::<CreditNote>(&format!(r#"{{"id": "1", "currency": "EUR", {extra}}}"#))
    };
    assert!(read(r#""showNet": "2""#).is_err());
    assert_eq!(
        read(r#""bookingCategory": "ROYALTY_ASSIGNED""#)
            .unwrap()
            .booking_category,
        Some(CreditNoteBookingCategory::RoyaltyAssigned)
    );
    assert_eq!(
        read(r#""bookingCategory": "ZZ""#).unwrap().booking_category,
        Some(CreditNoteBookingCategory::Other("ZZ".into()))
    );
    assert_eq!(
        read(r#""sendType": "0""#).unwrap().send_type,
        Some(DocumentSendType::Unset)
    );
    // Measured only in the shape survey: a reference to an accounting type.
    assert_eq!(
        read(r#""accountingType": {"id": "46", "objectName": "AccountingType"}"#)
            .unwrap()
            .accounting_type,
        Some(ObjectRef::new(AccountingTypeId::new(46)))
    );
}

#[test]
fn a_gross_credit_note_position_reads_eight_digit_unit_prices() {
    // Synthetic: the form (strings, up to eight fraction digits in `priceNet`
    // and `priceTax`) is what the shape survey measured on gross credit notes.
    let p: CreditNotePosition = serde_json::from_str(
        r#"{"id": "1", "quantity": "2.5", "price": "11.9", "priceNet": "8.40336134",
            "priceGross": "10", "priceTax": "1.59663866", "discount": "10",
            "isPercentage": "0", "discountedValue": "10", "optional": true}"#,
    )
    .unwrap();
    assert_eq!(p.quantity, Some(Decimal::new(25, 1)));
    assert_eq!(p.price_net, Some(Decimal::new(840336134, 8)));
    assert_eq!(p.price_tax, Some(Decimal::new(159663866, 8)));
    assert_eq!(p.is_percentage, Some(false));
    assert_eq!(p.discounted_value, Some(Decimal::new(10, 0)));
    assert_eq!(p.optional, Some(true));
}

#[tokio::test]
async fn sample_credit_note_list_reads_every_measured_form() {
    let (_s, c) = serving("/CreditNote", "sample_credit_note_list").await;
    let list = c
        .list_credit_notes(&CreditNoteQuery::default())
        .await
        .unwrap();
    assert_eq!(list.len(), 4);
    let by_id = |id: i64| list.iter().find(|n| n.id == CreditNoteId::new(id)).unwrap();

    let linked = by_id(2995842);
    assert_eq!(linked.credit_note_number.as_deref(), Some("GU-1004"));
    assert_eq!(linked.status, Some(CreditNoteStatus::Open));
    assert_eq!(
        linked.booking_category,
        Some(CreditNoteBookingCategory::Underachievement)
    );
    assert_eq!(linked.send_type, Some(DocumentSendType::Vpdf));
    assert_eq!(linked.sum_net, Some(eur(900)));
    assert_eq!(linked.sum_tax, Some(eur(171)));
    assert_eq!(linked.sum_gross, Some(eur(1071)));
    assert_eq!(linked.show_net, Some(true));
    assert_eq!(linked.is_transferred, Some(false));
    assert_eq!(
        linked.ref_src_invoice,
        Some(ObjectRef::new(InvoiceId::new(135171669)))
    );
    assert!(linked.origin.is_none());
    assert!(linked.accounting_type.is_none());

    let provision = by_id(2998960);
    assert_eq!(
        provision.booking_category,
        Some(CreditNoteBookingCategory::Provision)
    );
    assert_eq!(
        provision.accounting_type,
        Some(ObjectRef::new(AccountingTypeId::new(32891)))
    );
    assert_eq!(provision.tax_rule, Some(ObjectRef::new(TaxRuleId::new(9))));
    assert_eq!(provision.show_net, Some(false));
    assert_eq!(provision.sum_net, Some(eur(336)));
    assert_eq!(provision.sum_gross, Some(eur(400)));

    let draft = by_id(3079457);
    assert_eq!(draft.status, Some(CreditNoteStatus::Draft));
    assert_eq!(draft.send_type, Some(DocumentSendType::Unset));
    assert_eq!(draft.enshrined, None);
    assert_eq!(draft.origin.unwrap().id, InvoiceId::new(138602752));
    assert_eq!(draft.sum_gross, Some(eur(1190)));

    let from_invoice = by_id(3083917);
    assert_eq!(from_invoice.status, Some(CreditNoteStatus::Open));
    assert_eq!(from_invoice.sum_gross, Some(eur(23800)));
    assert_eq!(
        from_invoice.enshrined,
        Some(time::macros::datetime!(2026-11-07 1:18:15 +1))
    );
}

#[test]
fn sample_credit_note_pos_list_reads_every_measured_form() {
    let list = sample_positions::<CreditNotePosition>("sample_credit_note_pos_list");
    assert_eq!(list.len(), 4);
    let by_id = |id: i64| {
        list.iter()
            .find(|p| p.id == CreditNotePositionId::new(id))
            .unwrap()
    };

    let discounted = by_id(6134606);
    assert_eq!(discounted.discount, Some(Decimal::new(20, 0)));
    assert_eq!(discounted.discounted_value, Some(Decimal::new(20, 0)));
    assert_eq!(discounted.sum_discount, Some(eur(980)));
    assert_eq!(discounted.sum_gross, Some(eur(4665)));
    assert_eq!(discounted.sum_gross_accounting, Some(eur(0)));
    assert_eq!(
        discounted.credit_note,
        Some(ObjectRef::new(CreditNoteId::new(2996042)))
    );

    // Gross positions carry eight fraction digits in `priceNet`/`priceTax`.
    let gross = by_id(6140013);
    assert_eq!(gross.quantity, Some(Decimal::new(1, 0)));
    assert_eq!(gross.price, Some(Decimal::new(4, 0)));
    assert_eq!(gross.price_net, Some(Decimal::new(336134454, 8)));
    assert_eq!(gross.price_tax, Some(Decimal::new(63865546, 8)));
    assert_eq!(gross.position_number, Some(0));
    assert_eq!(gross.sum_net, Some(eur(336)));

    let texted = by_id(6383119);
    assert_eq!(
        texted.text.as_deref(),
        Some("SDRS-LEN-1795338407 [0020]„Tarif ä“ [003")
    );
    assert_eq!(texted.is_percentage, Some(true));
    assert_eq!(
        texted.create,
        Some(time::macros::datetime!(2026-11-22 10:06:58 +1))
    );

    let multi = by_id(6152387);
    assert_eq!(multi.quantity, Some(Decimal::new(3, 0)));
    assert_eq!(multi.price, Some(Decimal::new(1005, 2)));
    assert_eq!(multi.price_net, Some(Decimal::new(844537815, 8)));
    assert_eq!(multi.price_tax, Some(Decimal::new(160462185, 8)));
    assert_eq!(multi.sum_gross, Some(eur(3015)));
    assert_eq!(multi.tax_rate, Some(1900));
    assert_eq!(multi.temporary, Some(false));
}
