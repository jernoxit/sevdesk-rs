//! Every recorded fixture (real responses of the sevDesk test account, GET
//! only, headers not stored) read through the real client against a mock
//! server, plus the money edge cases.

#[path = "../common/mod.rs"]
mod common;

mod check_account;
mod contact;
mod credit_note;
mod invoice;
mod misc;
mod transaction;
mod voucher;

use common::{client, fixture};
use sevdesk::Amount;
use sevdesk::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn serving(route: &str, fixture_name: &str) -> (MockServer, SevdeskClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture(fixture_name)))
        .mount(&server)
        .await;
    let client = client(&server);
    (server, client)
}

fn eur(minor: i64) -> Amount {
    Amount::from_minor(minor)
}

// Shape-survey samples: up to four live objects per resource, scrubbed, chosen
// to cover every measured (key, form) pair.

fn sample_positions<T: serde::de::DeserializeOwned>(name: &str) -> Vec<T> {
    #[derive(serde::Deserialize)]
    struct Envelope<T> {
        objects: Vec<T>,
    }
    serde_json::from_str::<Envelope<T>>(&fixture(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .objects
}

fn assert_probe_address(
    name: Option<&str>,
    street: Option<&str>,
    zip: Option<&str>,
    city: Option<&str>,
    country: Option<ObjectRef<CountryId>>,
) {
    assert_eq!(name, Some("Probe Practice GmbH PROBE-R1"));
    assert_eq!(street, Some("Probestrasse 1"));
    assert_eq!(zip, Some("10115"));
    assert_eq!(city, Some("Berlin"));
    assert_eq!(country, Some(ObjectRef::new(CountryId::new(1))));
}
