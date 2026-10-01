//! Saving, reading and driving the lifecycle of invoices and credit notes against
//! a mock server. Most answers are synthetic: their shape is the measured one on
//! the test account (2026-09-29); the lifecycle calls follow the 2026-09-28 probe
//! log. `live_fixtures` reads the answers recorded on 2026-10-01.

#[path = "../common/mod.rs"]
mod common;
mod drafts;
mod lifecycle;
mod live_fixtures;
mod queries;
mod saving;

use common::{client, credit_note_draft, invoice_draft, position};
use serde_json::json;
use sevdesk::Amount;
use sevdesk::*;
use time::macros::datetime;
use wiremock::Request;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn eur(minor: i64) -> Amount {
    Amount::from_minor(minor)
}

fn answer(body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}
