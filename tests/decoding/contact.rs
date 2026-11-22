//! Fixture decoding of contacts, communication ways and contact addresses.

use super::{fixture, sample_positions, serving};
use sevdesk::*;

fn read_object<T: serde::de::DeserializeOwned>(name: &str) -> T {
    #[derive(serde::Deserialize)]
    struct Envelope<T> {
        objects: T,
    }
    serde_json::from_str::<Envelope<T>>(&fixture(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .objects
}

#[tokio::test]
async fn contact_reads_every_measured_form() {
    let (_s, c) = serving("/Contact/138615465", "contact").await;
    let contact = c.contact(ContactId::new(138615465)).await.unwrap();
    assert_eq!(
        contact.create,
        Some(time::macros::datetime!(2026-09-28 19:38:19 +2))
    );
    assert_eq!(
        contact.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert_eq!(contact.status, Some(ContactStatus::Active));
    assert_eq!(contact.customer_number.as_deref(), Some("1000"));
    assert_eq!(
        contact.category,
        Some(ObjectRef::new(ContactCategoryId::new(3)))
    );
    assert_eq!(contact.vat_number.as_deref(), Some("DE999999999"));
    assert_eq!(contact.exempt_vat, Some(false));
    assert_eq!(contact.default_discount_percentage, Some(true));
    assert_eq!(contact.government_agency, Some(false));
    assert_eq!(contact.default_show_vat, Some(true));
    // `null` in every measurement: the forms come from the spec.
    assert_eq!(contact.birthday, None);
    assert_eq!(contact.parent, None);
    assert_eq!(contact.default_cashback_time, None);
    assert_eq!(contact.additional_information, None);
}

#[test]
fn sample_contact_list_reads_every_measured_form() {
    let list = sample_positions::<Contact>("sample_contact_list");
    assert_eq!(list.len(), 3);
    // An organisation with a VAT number and a buyer reference.
    let org = &list[0];
    assert_eq!(org.vat_number.as_deref(), Some("REDACTED"));
    assert!(org.buyer_reference.is_some());
    assert_eq!(org.surename, None);
    // `surename` and `familyname` filled next to a `name`.
    let person = &list[1];
    assert_eq!(person.surename.as_deref(), Some("REDACTED"));
    assert_eq!(person.familyname.as_deref(), Some("REDACTED"));
    assert_eq!(person.vat_number, None);
    // A customer number that is text, and `""` read as `None`.
    let text = &list[2];
    assert_eq!(text.customer_number.as_deref(), Some("SDRS-SAMPLE-K-B2C"));
    assert_eq!(text.buyer_reference, None);
    assert_eq!(text.vat_number, None);
    assert!(
        list.iter()
            .all(|c| c.create.is_some() && c.update.is_some() && c.sev_client.is_some())
    );
}

#[test]
fn contact_spec_only_keys_read_their_spec_types() {
    // Synthetic: no measurement has these keys filled; the forms are the
    // OpenAPI spec's.
    let c: Contact = serde_json::from_str(
        r#"{"id":"1","parent":{"id":"7","objectName":"Contact"},"birthday":"1990-01-31",
        "gender":"w","academicTitle":"Dr.","name2":"M.","titel":"CEO",
        "bankAccount":"X","bankNumber":"Y","taxNumber":"T","taxOffice":"O",
        "defaultCashbackTime":"14","defaultCashbackPercent":"2.5",
        "defaultTimeToPay":"30","defaultDiscountAmount":"10.125",
        "additionalInformation":"info","status":"500"}"#,
    )
    .unwrap();
    assert_eq!(c.parent, Some(ObjectRef::new(ContactId::new(7))));
    assert_eq!(c.birthday, Some(time::macros::date!(1990 - 01 - 31)));
    assert_eq!(c.gender.as_deref(), Some("w"));
    assert_eq!(c.academic_title.as_deref(), Some("Dr."));
    assert_eq!(c.name2.as_deref(), Some("M."));
    assert_eq!(c.titel.as_deref(), Some("CEO"));
    assert_eq!(c.bank_account.as_deref(), Some("X"));
    assert_eq!(c.bank_number.as_deref(), Some("Y"));
    assert_eq!(c.tax_number.as_deref(), Some("T"));
    assert_eq!(c.tax_office.as_deref(), Some("O"));
    assert_eq!(c.default_cashback_time.as_deref(), Some("14"));
    assert_eq!(c.default_cashback_percent.as_deref(), Some("2.5"));
    assert_eq!(c.default_time_to_pay.as_deref(), Some("30"));
    assert_eq!(c.default_discount_amount.as_deref(), Some("10.125"));
    assert_eq!(c.additional_information.as_deref(), Some("info"));
    assert_eq!(c.status, Some(ContactStatus::Pending));
}

#[test]
fn contact_status_reads_spec_values_and_unknown_codes() {
    let status = |code: &str| {
        serde_json::from_str::<Contact>(&format!(r#"{{"id":"1","status":"{code}"}}"#))
            .unwrap()
            .status
    };
    // Synthetic: `100` and `500` are in the OpenAPI spec prose only.
    assert_eq!(status("100"), Some(ContactStatus::Lead));
    assert_eq!(status("500"), Some(ContactStatus::Pending));
    assert_eq!(status("1000"), Some(ContactStatus::Active));
    assert_eq!(status("123"), Some(ContactStatus::Other(123)));
}

#[test]
fn contact_birthday_in_another_form_is_a_decode_error() {
    assert!(
        serde_json::from_str::<Contact>(r#"{"id":"1","birthday":"1990-01-31T00:00:00+01:00"}"#)
            .is_err()
    );
}

#[test]
fn contact_address_reads_the_embedded_contact_as_a_reference() {
    let a: ContactAddress = read_object("contact_address");
    assert_eq!(a.id, ContactAddressId::new(120010251));
    assert_eq!(a.contact, Some(ObjectRef::new(ContactId::new(138615465))));
    assert_eq!(
        a.create,
        Some(time::macros::datetime!(2026-09-28 19:38:20 +2))
    );
    assert_eq!(
        a.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert_eq!(a.zip.as_deref(), Some("10115"));
    assert_eq!(a.country, Some(ObjectRef::new(CountryId::new(1))));
    // `null` in every measurement.
    assert_eq!(a.name, None);
}

#[test]
fn sample_contact_address_list_reads_every_measured_form() {
    let list = sample_positions::<ContactAddress>("sample_contact_address_list");
    assert_eq!(list.len(), 1);
    let a = &list[0];
    assert_eq!(a.category, Some(ObjectRef::new(AddressCategoryId::new(43))));
    assert_eq!(a.city.as_deref(), Some("Berlin"));
    assert_eq!(a.street.as_deref(), Some("REDACTED"));
    assert!(a.create.is_some() && a.update.is_some() && a.sev_client.is_some());
    assert_eq!((&a.name2, &a.name3, &a.name4), (&None, &None, &None));
}

#[test]
fn contact_address_spec_only_keys_read_their_spec_types() {
    // Synthetic: `name2` to `name4` are in the OpenAPI spec only.
    let a: ContactAddress =
        serde_json::from_str(r#"{"id":"1","name":"A","name2":"B","name3":"C","name4":"D"}"#)
            .unwrap();
    assert_eq!(
        (
            a.name.as_deref(),
            a.name2.as_deref(),
            a.name3.as_deref(),
            a.name4.as_deref()
        ),
        (Some("A"), Some("B"), Some("C"), Some("D"))
    );
}

#[test]
fn sample_communication_way_list_reads_every_measured_form() {
    let list = sample_positions::<CommunicationWay>("sample_communication_way_list");
    assert_eq!(list.len(), 1);
    let w = &list[0];
    assert_eq!(w.kind, Some(CommunicationWayType::Email));
    assert_eq!(w.main, Some(true));
    assert_eq!(w.value.as_deref(), Some("probe@example.com"));
    assert_eq!(w.key, Some(ObjectRef::new(CommunicationWayKeyId::new(2))));
    assert_eq!(w.contact, Some(ObjectRef::new(ContactId::new(138615465))));
    assert_eq!(
        w.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert!(w.create.is_some() && w.update.is_some());
}

#[test]
fn communication_way_type_spec_only_values_read() {
    // Synthetic: `PHONE`, `WEB` and `MOBILE` are in the OpenAPI spec only.
    let kind = |text: &str| {
        serde_json::from_str::<CommunicationWay>(&format!(r#"{{"id":"1","type":"{text}"}}"#))
            .unwrap()
            .kind
    };
    assert_eq!(kind("PHONE"), Some(CommunicationWayType::Phone));
    assert_eq!(kind("WEB"), Some(CommunicationWayType::Web));
    assert_eq!(kind("MOBILE"), Some(CommunicationWayType::Mobile));
    assert_eq!(
        kind("FAX"),
        Some(CommunicationWayType::Other("FAX".to_owned()))
    );
}
