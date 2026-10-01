//! Transport behaviour (errors, retries, pacing, drift guards) against a mock server.
//! Write answers are real captures (`fixtures/write_*.json`, recorded by
//! `live_write_6_record_raw_write_bodies`); bodies marked "synthetic" are built
//! from the measured protocol instead.

mod common;

use common::{TEST_TOKEN, client, client_for, fixture, new_transaction};
use sevdesk::*;
use std::time::{Duration, Instant};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn token_never_shows_in_debug_or_display() {
    let config = SevdeskClientConfig::new(ApiToken::new(TEST_TOKEN));
    let shown = format!("{config:?} {} {:?}", config.api_token, config.api_token);
    assert!(!shown.contains(TEST_TOKEN), "{shown}");
}

#[tokio::test]
async fn sends_raw_token_accept_version_and_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Tools/bookkeepingSystemVersion"))
        .and(header("Authorization", TEST_TOKEN))
        .and(header("Accept", "application/json"))
        .and(header("X-Version", DEFAULT_X_VERSION))
        .and(header("User-Agent", "sevdesk-rs (jernoxit)"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("bookkeeping_system_version")),
        )
        .expect(1)
        .mount(&server)
        .await;
    client(&server).bookkeeping_system_version().await.unwrap();
}

#[tokio::test]
async fn error_object_in_http_200_is_an_error() {
    // Synthetic: sevDesk answers the storno of a partially paid invoice with
    // 200 and code 390 (measured); the body shape is that of every error.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"error":{"message":"not allowed","code":390,"responseCode":400},"objects":null}"#,
        ))
        .mount(&server)
        .await;
    let err = client(&server)
        .invoice(InvoiceId::new(1))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            SevdeskError::Api {
                status: 400,
                code: Some(390),
                ..
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn known_codes_are_data() {
    for (code, known) in [
        (750, KnownCode::InvoiceAlreadyPaid),
        (751, KnownCode::VoucherAlreadyPaid),
        (421, KnownCode::AlreadyCancelled),
        (170, KnownCode::CreditNoteExists),
        (159, KnownCode::NotDeletable),
        (390, KnownCode::PartlyPaid),
        (455, KnownCode::CreditSumExceedsInvoice),
        (456, KnownCode::TaxRateDiffersFromOrigin),
        (830, KnownCode::EInvoiceFieldsMissing),
        (7100, KnownCode::NegativeInvoiceSum),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(400).set_body_string(format!(
                r#"{{"error":{{"message":"m","code":{code},"responseCode":400}},"objects":null}}"#
            )))
            .mount(&server)
            .await;
        let err = client(&server)
            .reset_voucher_to_open(VoucherId::new(1))
            .await
            .unwrap_err();
        assert_eq!(err.known_code(), Some(known), "{err:?}");
    }
}

#[tokio::test]
async fn non_json_5xx_is_an_api_error_with_the_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>bad gateway</html>"))
        .mount(&server)
        .await;
    let err = client(&server).list_check_accounts().await.unwrap_err();
    assert!(
        matches!(err, SevdeskError::Api { status: 502, .. }),
        "{err:?}"
    );
}

async fn serve_body(body: &str) -> (MockServer, SevdeskClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let c = client(&server);
    (server, c)
}

#[tokio::test]
async fn drift_guard_rejects_objects_without_their_load_bearing_fields() {
    // An id and a currency and nothing else: never a defaulted voucher.
    let (_s, c) =
        serve_body(r#"{"objects":[{"id":"1","objectName":"Voucher","currency":"EUR"}]}"#).await;
    let err = c.voucher(VoucherId::new(1)).await.unwrap_err();
    assert!(
        matches!(
            err,
            SevdeskError::DriftGuard {
                context: "Voucher",
                ..
            }
        ),
        "{err:?}"
    );

    // No `objects`, `null` objects, empty list for a by-id read.
    for body in [r#"{}"#, r#"{"objects":null}"#, r#"{"objects":[]}"#] {
        let (_s, c) = serve_body(body).await;
        let err = c.voucher(VoucherId::new(1)).await.unwrap_err();
        assert!(
            matches!(err, SevdeskError::DriftGuard { .. }),
            "{body}: {err:?}"
        );
    }

    // Another shape entirely: no id.
    let (_s, c) = serve_body(r#"{"objects":[{"foo":"bar"}]}"#).await;
    let err = c.voucher(VoucherId::new(1)).await.unwrap_err();
    assert!(matches!(err, SevdeskError::Decode { .. }), "{err:?}");
}

#[tokio::test]
async fn unknown_status_and_type_codes_stay_readable() {
    let (_s, c) = serve_body(
        r#"{"objects":[{"id":"1","status":"777","invoiceType":"XX","sumGross":"1.50","currency":"EUR","newField":1}]}"#,
    )
    .await;
    let i = c.invoice(InvoiceId::new(1)).await.unwrap();
    assert_eq!(i.status, Some(InvoiceStatus::Other(777)));
    assert_eq!(i.invoice_type, Some(InvoiceType::Other("XX".into())));
}

#[tokio::test]
async fn retries_429_with_retry_after_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("bookkeeping_system_version")),
        )
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).bookkeeping_system_version().await.unwrap(),
        "2.0"
    );
}

#[tokio::test]
async fn persistent_429_is_rate_limited_after_bounded_attempts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .expect(8)
        .mount(&server)
        .await;
    let err = client(&server).list_check_accounts().await.unwrap_err();
    assert!(
        matches!(err, SevdeskError::RateLimited { attempts: 8 }),
        "{err:?}"
    );
}

#[tokio::test]
async fn requests_are_spaced_by_the_minimum_interval() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("bookkeeping_system_version")),
        )
        .mount(&server)
        .await;
    let fixed = Duration::from_millis(300);
    let c = client_for(
        &server,
        Pacing {
            initial_interval: fixed,
            min_interval: fixed,
            max_interval: fixed,
        },
    );
    let start = Instant::now();
    c.bookkeeping_system_version().await.unwrap();
    c.bookkeeping_system_version().await.unwrap();
    c.bookkeeping_system_version().await.unwrap();
    assert!(
        start.elapsed() >= Duration::from_millis(600),
        "{:?}",
        start.elapsed()
    );
}

#[tokio::test]
async fn a_429_on_a_write_is_sent_again_and_never_read_back() {
    // Owner decision 2026-09-30: a 429 was not processed.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(201).set_body_string(fixture("write_transaction_create")),
        )
        .mount(&server)
        .await;
    client(&server)
        .create_transaction(&new_transaction())
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "one 429, one retry, no read");
    assert!(requests.iter().all(|r| r.method.as_str() == "POST"));
    assert_eq!(requests[0].body, requests[1].body, "the same write twice");
}

#[tokio::test]
async fn other_write_errors_are_not_retried() {
    for status in [400, 500] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_string("{}"))
            .mount(&server)
            .await;
        let err = client(&server)
            .create_transaction(&new_transaction())
            .await
            .unwrap_err();
        assert!(matches!(err, SevdeskError::Api { .. }), "{err:?}");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn retry_after_is_honoured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("bookkeeping_system_version")),
        )
        .mount(&server)
        .await;
    let start = Instant::now();
    client(&server).bookkeeping_system_version().await.unwrap();
    assert!(
        start.elapsed() >= Duration::from_secs(1),
        "{:?}",
        start.elapsed()
    );
}

#[tokio::test]
async fn the_interval_grows_on_429_and_shrinks_after_successes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(fixture("bookkeeping_system_version")),
        )
        .mount(&server)
        .await;
    let c = client_for(
        &server,
        Pacing {
            initial_interval: Duration::from_millis(60),
            min_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(500),
        },
    );
    c.bookkeeping_system_version().await.unwrap();
    // One 429 doubled it; the success after it is the first of ten.
    assert_eq!(c.request_interval(), Duration::from_millis(120));
    for _ in 0..9 {
        c.bookkeeping_system_version().await.unwrap();
    }
    assert_eq!(c.request_interval(), Duration::from_millis(70));
}
