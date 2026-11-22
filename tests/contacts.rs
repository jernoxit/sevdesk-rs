//! Contact, address and communication-way requests (serialized body equals the
//! sevDesk body, `None` fields absent) and the answers. Fixtures are real
//! captures of the test account (probe log 2026-09-28, lines 237, 13, 14).

mod common;

use common::{client, fixture};
use serde_json::{Value, json};
use sevdesk::*;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(
    clippy::expect_used,
    reason = "a helper, not a test fn: a failure is a test failure"
)]
fn body(v: &impl serde::Serialize) -> Value {
    serde_json::to_value(v).expect("serializes")
}

fn ok(text: String) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_string(text)
}

#[test]
fn new_organisation_contact_body() {
    let mut c = NewContact::new(ContactName::Organisation {
        name: "Praxis GmbH".into(),
    });
    assert_eq!(
        body(&c),
        json!({"name": "Praxis GmbH", "category": {"id": 3, "objectName": "Category"}, "status": 1000})
    );
    c.options.buyer_reference = Some("-".into());
    c.options.vat_number = Some("DE999999999".into());
    c.options.customer_number = Some("K-1".into());
    assert_eq!(
        body(&c),
        json!({
            "name": "Praxis GmbH",
            "category": {"id": 3, "objectName": "Category"},
            "status": 1000,
            "buyerReference": "-",
            "vatNumber": "DE999999999",
            "customerNumber": "K-1"
        })
    );
}

#[test]
fn new_person_contact_body() {
    let c = NewContact::new(ContactName::Person {
        surename: "Probe".into(),
        familyname: "Halter".into(),
    });
    assert_eq!(
        body(&c),
        json!({"surename": "Probe", "familyname": "Halter",
               "category": {"id": 3, "objectName": "Category"}, "status": 1000})
    );
}

#[test]
fn contact_update_sends_only_what_is_set() {
    assert_eq!(body(&ContactUpdate::default()), json!({}));
    let u = ContactUpdate {
        buyer_reference: Some("KREF-1".into()),
        vat_number: Some("DE1".into()),
        customer_number: Some("7".into()),
        name: Some("N".into()),
        surename: Some("S".into()),
        familyname: Some("F".into()),
        ..Default::default()
    };
    assert_eq!(
        body(&u),
        json!({"name": "N", "surename": "S", "familyname": "F",
               "buyerReference": "KREF-1", "vatNumber": "DE1", "customerNumber": "7"})
    );
}

#[test]
fn new_contact_address_body_matches_the_measured_one() {
    let a = NewContactAddress {
        contact: ObjectRef::new(ContactId::new(138615465)),
        street: "Probestrasse 1".into(),
        zip: "10115".into(),
        city: "Berlin".into(),
        country: ObjectRef::new(GERMANY),
        category: ObjectRef::new(AddressCategoryId::new(43)),
        options: Default::default(),
    };
    assert_eq!(
        body(&a),
        json!({
            "contact": {"id": 138615465, "objectName": "Contact"},
            "street": "Probestrasse 1", "zip": "10115", "city": "Berlin",
            "country": {"id": 1, "objectName": "StaticCountry"},
            "category": {"id": 43, "objectName": "Category"}
        })
    );
}

#[test]
fn contact_address_update_sends_only_what_is_set() {
    assert_eq!(body(&ContactAddressUpdate::default()), json!({}));
    let u = ContactAddressUpdate {
        street: Some("S".into()),
        zip: Some("Z".into()),
        city: Some("C".into()),
        country: Some(ObjectRef::new(GERMANY)),
        category: Some(ObjectRef::new(AddressCategoryId::new(47))),
        ..Default::default()
    };
    assert_eq!(
        body(&u),
        json!({"street": "S", "zip": "Z", "city": "C",
               "country": {"id": 1, "objectName": "StaticCountry"},
               "category": {"id": 47, "objectName": "Category"}})
    );
}

#[test]
fn new_communication_way_body_matches_the_measured_one() {
    let w = NewCommunicationWay::email(
        ContactId::new(138615465),
        "probe@example.com",
        CommunicationWayKeyId::new(2),
        true,
    );
    assert_eq!(
        body(&w),
        json!({
            "contact": {"id": 138615465, "objectName": "Contact"},
            "type": "EMAIL", "value": "probe@example.com",
            "key": {"id": 2, "objectName": "CommunicationWayKey"}, "main": true
        })
    );
}

#[test]
fn communication_way_update_sends_only_what_is_set() {
    assert_eq!(body(&CommunicationWayUpdate::default()), json!({}));
    let u = CommunicationWayUpdate {
        value: Some("a@example.com".into()),
        key: Some(ObjectRef::new(CommunicationWayKeyId::new(8))),
        main: Some(false),
        ..Default::default()
    };
    assert_eq!(
        body(&u),
        json!({"value": "a@example.com",
               "key": {"id": 8, "objectName": "CommunicationWayKey"}, "main": false})
    );
}

#[tokio::test]
async fn contact_by_id_decodes_the_measured_b2b_contact() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Contact/138615465"))
        .respond_with(ok(fixture("contact")))
        .mount(&server)
        .await;
    let c = client(&server)
        .contact(ContactId::new(138615465))
        .await
        .unwrap();
    assert_eq!(c.name.as_deref(), Some("Probe Practice GmbH PROBE-R1"));
    assert_eq!(c.customer_number.as_deref(), Some("1000"));
    assert_eq!(
        c.buyer_reference.as_deref(),
        Some("PROBE-R1-LEITWEG-04011000-1234512345-06")
    );
    assert_eq!(c.vat_number.as_deref(), Some("DE999999999"));
    assert_eq!(c.surename, None);
}

#[tokio::test]
async fn a_person_contact_without_name_passes_the_guard() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Contact/138615466"))
        .respond_with(ok(json!({"objects": [{
            "id": "138615466", "name": null, "customerNumber": "1001",
            "surename": "Probe", "familyname": "Halter PROBE-R1"
        }]})
        .to_string()))
        .mount(&server)
        .await;
    let c = client(&server)
        .contact(ContactId::new(138615466))
        .await
        .unwrap();
    assert_eq!(c.surename.as_deref(), Some("Probe"));
    assert_eq!(c.buyer_reference, None);
}

#[tokio::test]
async fn contact_without_any_load_bearing_field_is_a_drift_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Contact/1"))
        .respond_with(ok(json!({"objects": [{"id": "1"}]}).to_string()))
        .mount(&server)
        .await;
    let err = client(&server)
        .contact(ContactId::new(1))
        .await
        .unwrap_err();
    assert!(matches!(err, SevdeskError::DriftGuard { .. }), "{err:?}");
}

#[tokio::test]
async fn customer_number_lookup_compares_exactly_on_the_result() {
    let server = MockServer::start().await;
    // A substring or ignored filter answers more than the exact match.
    Mock::given(method("GET"))
        .and(path("/Contact"))
        .respond_with(ok(json!({"objects": [
            {"id": "1", "name": "A", "customerNumber": "1000"},
            {"id": "2", "name": "B", "customerNumber": "100"}
        ]})
        .to_string()))
        .mount(&server)
        .await;
    let c = client(&server);
    let found = c.find_contact_by_customer_number("100").await.unwrap();
    assert_eq!(found.unwrap().id, ContactId::new(2));
    assert!(
        c.find_contact_by_customer_number("10")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn create_contact_address_decodes_the_measured_answer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ContactAddress"))
        .and(body_json(json!({
            "contact": {"id": 138615465, "objectName": "Contact"},
            "street": "Probestrasse 1", "zip": "10115", "city": "Berlin",
            "country": {"id": 1, "objectName": "StaticCountry"},
            "category": {"id": 43, "objectName": "Category"}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_string(fixture("contact_address")))
        .expect(1)
        .mount(&server)
        .await;
    let a = client(&server)
        .create_contact_address(&NewContactAddress {
            contact: ObjectRef::new(ContactId::new(138615465)),
            street: "Probestrasse 1".into(),
            zip: "10115".into(),
            city: "Berlin".into(),
            country: ObjectRef::new(GERMANY),
            category: ObjectRef::new(AddressCategoryId::new(43)),
            options: Default::default(),
        })
        .await
        .unwrap();
    assert_eq!(a.id, ContactAddressId::new(120010251));
    assert_eq!(a.contact.unwrap().id, ContactId::new(138615465));
    assert_eq!(a.street.as_deref(), Some("Probestrasse 1"));
    assert_eq!(a.zip.as_deref(), Some("10115"));
    assert_eq!(a.city.as_deref(), Some("Berlin"));
    assert_eq!(a.country.unwrap().id, GERMANY);
    assert_eq!(a.category.unwrap().id, AddressCategoryId::new(43));
}

#[tokio::test]
async fn create_communication_way_decodes_the_measured_answer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/CommunicationWay"))
        .respond_with(ResponseTemplate::new(201).set_body_string(fixture("communication_way")))
        .expect(1)
        .mount(&server)
        .await;
    let w = client(&server)
        .create_communication_way(&NewCommunicationWay::email(
            ContactId::new(138615465),
            "probe@example.com",
            CommunicationWayKeyId::new(2),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(w.id, CommunicationWayId::new(115284944));
    assert_eq!(w.kind, Some(CommunicationWayType::Email));
    assert_eq!(w.value.as_deref(), Some("probe@example.com"));
    assert_eq!(w.key.unwrap().id, CommunicationWayKeyId::new(2));
    assert_eq!(w.main, Some(true));
}

#[tokio::test]
async fn updates_use_put_on_the_resource_path() {
    let server = MockServer::start().await;
    for (route, answer) in [
        ("/Contact/5", json!({"objects": {"id": "5", "name": "N"}})),
        (
            "/ContactAddress/6",
            json!({"objects": {"id": "6", "city": "Berlin"}}),
        ),
        (
            "/CommunicationWay/7",
            json!({"objects": {"id": "7", "value": "a@example.com"}}),
        ),
    ] {
        Mock::given(method("PUT"))
            .and(path(route))
            .respond_with(ok(answer.to_string()))
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);
    c.update_contact(ContactId::new(5), &ContactUpdate::default())
        .await
        .unwrap();
    c.update_contact_address(ContactAddressId::new(6), &ContactAddressUpdate::default())
        .await
        .unwrap();
    c.update_communication_way(
        CommunicationWayId::new(7),
        &CommunicationWayUpdate::default(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn listings_send_the_contact_filter_and_keep_only_that_contacts_rows() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ContactAddress"))
        .and(query_param("contact[id]", "9"))
        .and(query_param("contact[objectName]", "Contact"))
        .respond_with(ok(json!({"objects": [
            {"id": "1", "city": "Berlin", "contact": {"id": "9", "objectName": "Contact"}},
            {"id": "2", "city": "Bonn", "contact": {"id": "10", "objectName": "Contact"}}
        ]})
        .to_string()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/CommunicationWay"))
        .and(query_param("contact[id]", "9"))
        .and(query_param("contact[objectName]", "Contact"))
        .respond_with(ok(json!({"objects": [
            {"id": "1", "type": "EMAIL", "value": "a@example.com",
             "contact": {"id": "9", "objectName": "Contact"}},
            {"id": "2", "type": "EMAIL", "value": "b@example.com",
             "contact": {"id": "10", "objectName": "Contact"}}
        ]})
        .to_string()))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    let addresses = c.contact_addresses(ContactId::new(9)).await.unwrap();
    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses[0].id, ContactAddressId::new(1));
    let ways = c.communication_ways(ContactId::new(9)).await.unwrap();
    assert_eq!(ways.len(), 1);
    assert_eq!(ways[0].value.as_deref(), Some("a@example.com"));
}

#[test]
fn new_contact_with_all_options_sends_every_field_as_measured() {
    let mut c = NewContact::new(ContactName::Person {
        surename: "Probe".into(),
        familyname: "Halter".into(),
    });
    let o = &mut c.options;
    o.status = Some(ContactStatus::Pending);
    o.buyer_reference = Some("B".into());
    o.vat_number = Some("DE1".into());
    o.customer_number = Some("K".into());
    o.parent = Some(ObjectRef::new(ContactId::new(141661521)));
    o.titel = Some("Probe-Titel".into());
    o.description = Some("probe description".into());
    o.academic_title = Some("Dr.".into());
    o.gender = Some("m".into());
    o.name2 = Some("Probe-Name2".into());
    o.birthday = Some(time::macros::date!(1990 - 05 - 17));
    o.bank_account = Some("DE02120300000000202051".into());
    o.bank_number = Some("12030000".into());
    o.default_cashback_time = Some(14);
    o.default_cashback_percent_bp = Some(250);
    o.default_time_to_pay = Some(21);
    o.tax_number = Some("12/345/67890".into());
    o.tax_office = Some("Finanzamt Probe".into());
    o.exempt_vat = Some(true);
    o.default_discount_amount = Some(Amount::from_minor(500));
    o.government_agency = Some(true);
    assert_eq!(
        body(&c),
        json!({
            "surename": "Probe", "familyname": "Halter",
            "category": {"id": 3, "objectName": "Category"},
            "status": 500,
            "buyerReference": "B", "vatNumber": "DE1", "customerNumber": "K",
            "parent": {"id": 141661521, "objectName": "Contact"},
            "titel": "Probe-Titel", "description": "probe description",
            "academicTitle": "Dr.", "gender": "m", "name2": "Probe-Name2",
            "birthday": "1990-05-17",
            "bankAccount": "DE02120300000000202051", "bankNumber": "12030000",
            "defaultCashbackTime": 14, "defaultCashbackPercent": "2.50",
            "defaultTimeToPay": 21,
            "taxNumber": "12/345/67890", "taxOffice": "Finanzamt Probe",
            "exemptVat": true, "defaultDiscountAmount": "5.00",
            "governmentAgency": true
        })
    );
}

#[test]
fn contact_update_with_the_measured_fields_sends_them() {
    let u = ContactUpdate {
        status: Some(ContactStatus::Pending),
        category: Some(ObjectRef::new(ContactCategoryId::new(2))),
        parent: Some(ObjectRef::new(ContactId::new(9))),
        titel: Some("T".into()),
        description: Some("D".into()),
        academic_title: Some("Dr.".into()),
        gender: Some("m".into()),
        name2: Some("N2".into()),
        birthday: Some(time::macros::date!(1990 - 05 - 17)),
        bank_account: Some("DE02".into()),
        bank_number: Some("1".into()),
        default_cashback_time: Some(14),
        default_cashback_percent_bp: Some(250),
        default_time_to_pay: Some(21),
        tax_number: Some("TN".into()),
        tax_office: Some("TO".into()),
        exempt_vat: Some(true),
        default_discount_amount: Some(Amount::from_minor(500)),
        government_agency: Some(false),
        ..Default::default()
    };
    assert_eq!(
        body(&u),
        json!({
            "category": {"id": 2, "objectName": "Category"},
            "status": 500,
            "parent": {"id": 9, "objectName": "Contact"},
            "titel": "T", "description": "D", "academicTitle": "Dr.",
            "gender": "m", "name2": "N2", "birthday": "1990-05-17",
            "bankAccount": "DE02", "bankNumber": "1",
            "defaultCashbackTime": 14, "defaultCashbackPercent": "2.50",
            "defaultTimeToPay": 21, "taxNumber": "TN", "taxOffice": "TO",
            "exemptVat": true, "defaultDiscountAmount": "5.00",
            "governmentAgency": false
        })
    );
}

#[test]
fn address_name_and_communication_way_type_are_sent_when_set() {
    let mut a = NewContactAddress {
        contact: ObjectRef::new(ContactId::new(1)),
        street: "S".into(),
        zip: "Z".into(),
        city: "C".into(),
        country: ObjectRef::new(GERMANY),
        category: ObjectRef::new(AddressCategoryId::new(43)),
        options: Default::default(),
    };
    a.options.name = Some("Probe Name".into());
    assert_eq!(body(&a)["name"], json!("Probe Name"));
    let u = ContactAddressUpdate {
        name: Some("Probe Name".into()),
        ..Default::default()
    };
    assert_eq!(body(&u), json!({"name": "Probe Name"}));
    let w = CommunicationWayUpdate {
        kind: Some(CommunicationWayType::Phone),
        ..Default::default()
    };
    assert_eq!(body(&w), json!({"type": "PHONE"}));
}
