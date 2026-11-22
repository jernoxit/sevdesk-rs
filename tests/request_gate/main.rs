//! The request gate: the write-side release check of the crate. Every object is
//! created and updated through the crate's OWN public typed client and request
//! types (no raw JSON bodies), then read back RAW and compared field by field
//! with the body the crate sent, so the probe proves the crate's encodings.
//! Writes PERMANENT objects to the sevDesk TEST account (the probe aborts on
//! any other account before the first write), so it is its own test target
//! (`test = false` in Cargo.toml) and runs only on request:
//!
//! `just test-live --test request_gate case1`
//!
//! Part 1: one object per type, created with EVERY field of its `options`
//! struct set (invoice draft incl. the deprecated `send_date`, `send_type` and
//! a custom `invoice_number` with the run marker, credit note draft, voucher
//! draft with a gross position, contact, address, communication way,
//! transaction), then the updates with every field of the `*Update` types.
//! After each write the object (and its positions) is read back and every
//! field the crate sent is classed `stored` / `ignored` / `changed` /
//! `refused`. Output: `target/tmp/request_gate/<object>.tsv`, stderr
//! `GATE\t<object>\t<route>\t<field>\t<class>`.
//!
//! Part 2 (read-only): every query parameter that `e104b68` added is sent once
//! through the crate's list methods with a value taken from live data; the
//! call must decode, carry no `error`, and be non-empty / narrower than the
//! unfiltered list where that is expected. Output
//! `target/tmp/request_gate/queries.tsv`, stderr
//! `GATEQ\t<query>\t<param>\t<class>\t<n>/<unfiltered>` with the classes
//! `ok` / `empty` / `not-narrowed` / `error` (and `no-data` when the account
//! has nothing to take the value from).
//!
//! Budget: 7 permanent objects (invoice draft, credit note draft, voucher
//! draft, contact, address, communication way, transaction) and ZERO consumed
//! document numbers: the invoice carries a custom number with the run marker,
//! the credit note the number the sequence names next but only as a draft; the
//! `SevSequence` is read before and after (`NUMBER-CHECK`). Every object prints
//! `CREATED <kind> <id>`. No write is retried: after a failed write the answer
//! may still have been written, so the run stops and names the marker to look
//! up.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness files are shared with live_write and use only some of the helpers; a failure in a test helper is a test failure"
)]

#[path = "../live_write/docs.rs"]
mod docs;
mod documents;
mod existing;
mod gate;
#[path = "../live_write/harness.rs"]
mod harness;
mod parties;
mod queries;
#[path = "../common/scrub.rs"]
mod scrub;
mod setup;

use docs::raw_one;
use gate::{Recorder, compare};
use harness::*;
use reqwest::Method;
use serde_json::Value;
use setup::Inputs;
use std::collections::BTreeMap;
use time::OffsetDateTime;

const BUDGET_OBJECTS: usize = 7;

/// What the queries of part 2 take their values from: the objects as read back
/// after their last write.
pub(crate) struct Facts {
    pub(crate) invoice: Value,
    pub(crate) invoice_number: String,
    pub(crate) credit_note: Value,
    pub(crate) voucher: Value,
    pub(crate) voucher_pos: Value,
    pub(crate) contact: Value,
    pub(crate) address: Value,
    pub(crate) transaction: Value,
}

pub(crate) struct Gate<'a> {
    pub(crate) ctx: &'a Ctx,
    pub(crate) rec: Recorder,
    pub(crate) inp: Inputs,
    created: usize,
    /// `nextSequence` before the first write, per `<objectType>/<type>`.
    sequences: BTreeMap<&'static str, String>,
    pub(crate) number_ok: bool,
}

impl<'a> Gate<'a> {
    /// Counts and prints a permanent object after it was written.
    pub(crate) fn created(&mut self, what: &str, id: i64) {
        self.created += 1;
        eprintln!("CREATED {what} {id}");
    }

    /// Stops before the write that would exceed the budget.
    pub(crate) fn may_create(&self) {
        assert!(self.created < BUDGET_OBJECTS, "object budget exhausted");
    }

    /// A failed create is no proof of "not written": stop, never retry.
    pub(crate) fn create_failed(&self, what: &str, error: &dyn std::fmt::Display) -> ! {
        eprintln!(
            "WRITE-FAILED {what}: {error}; sevDesk may have written it anyway: look up marker {} before any retry",
            self.ctx.marker
        );
        panic!("{what} failed: {error}");
    }

    /// `GET <path>` raw; a failing read is no measurement: panic.
    pub(crate) async fn get_one(&self, path: &str) -> Value {
        let (status, text) = self.ctx.raw(Method::GET, path, None, None).await;
        assert!(status < 300, "GET {path} answered {status}");
        raw_one(&text)
    }

    pub(crate) async fn get_list(&self, path: &str) -> Vec<Value> {
        let (status, text) = self.ctx.raw(Method::GET, path, None, None).await;
        assert!(status < 300, "GET {path} answered {status}");
        match docs::raw_objects(&text) {
            Value::Array(list) => list,
            other => vec![other],
        }
    }

    /// Reads the object at `path` and compares what the crate `sent`.
    pub(crate) async fn check(
        &mut self,
        object: &str,
        route: &str,
        sent: &Value,
        before: Option<&Value>,
        path: &str,
        failed: bool,
    ) -> Value {
        let got = self.get_one(path).await;
        compare(&mut self.rec, object, route, sent, before, &got, failed);
        got
    }

    /// Reads the first position at `path` and compares what the crate `sent`.
    pub(crate) async fn check_pos(
        &mut self,
        object: &str,
        route: &str,
        sent: &Value,
        before: Option<&Value>,
        path: &str,
    ) -> Value {
        let list = self.get_list(path).await;
        assert_eq!(list.len(), 1, "GET {path}: expected the one position");
        compare(&mut self.rec, object, route, sent, before, &list[0], false);
        list[0].clone()
    }

    /// An update through the crate: the answer decides `failed`, the re-read
    /// decides what happened.
    pub(crate) async fn update_step<T>(
        &mut self,
        object: &str,
        route: &str,
        sent: Value,
        before: &Value,
        path: &str,
        result: Result<T, sevdesk::SevdeskError>,
    ) -> Value {
        let failed = result.is_err();
        if let Err(e) = &result {
            eprintln!("UPDATE-FAILED\t{object}\t{route}\t{e}");
        }
        self.check(object, route, &sent, Some(before), path, failed)
            .await
    }

    /// `nextSequence` of a sequence, read-only.
    pub(crate) async fn next_sequence(&self, object_type: &str, kind: &str) -> String {
        let head = self
            .get_one(&format!(
                "/SevSequence/Factory/getByType?objectType={object_type}&type={kind}"
            ))
            .await;
        head["nextSequence"].as_str().unwrap_or("?").to_owned()
    }

    /// After a draft was saved: the sequence did not move and the draft holds
    /// the number the caller chose, so no sequence number was consumed.
    pub(crate) async fn number_check(
        &mut self,
        object_type: &'static str,
        kind: &str,
        read_back: &str,
        expected: &str,
    ) {
        let before = self.sequences[object_type].clone();
        let after = self.next_sequence(object_type, kind).await;
        let ok = before == after && read_back == expected;
        eprintln!(
            "NUMBER-CHECK\t{object_type}\tsequence before={before}\tafter={after}\tnumber read back={read_back}\texpected={expected}\tok={ok}"
        );
        self.number_ok &= ok;
    }
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_request_gate() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the request gate");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-GATE-{ts}"));
    setup::verify_test_account(&ctx).await;
    let inp = Inputs::read(&ctx).await;
    let mut g = Gate {
        ctx: &ctx,
        rec: Recorder::new(),
        inp,
        created: 0,
        sequences: BTreeMap::new(),
        number_ok: true,
    };
    for (object_type, kind) in [("Invoice", "RE"), ("CreditNote", "CN")] {
        let next = g.next_sequence(object_type, kind).await;
        eprintln!("SEQUENCE\t{object_type}\tbefore the first write: nextSequence {next}");
        g.sequences.insert(object_type, next);
    }
    eprintln!("PLAN objects 7 (budget {BUDGET_OBJECTS}), consumed numbers 0");

    let (invoice, invoice_number) = documents::invoice(&mut g).await;
    let credit_note = documents::credit_note(&mut g).await;
    let (voucher, voucher_pos) = documents::voucher(&mut g).await;
    let (contact, address, transaction) = parties::all(&mut g).await;
    let facts = Facts {
        invoice,
        invoice_number,
        credit_note,
        voucher,
        voucher_pos,
        contact,
        address,
        transaction,
    };
    eprintln!("DONE objects created: {}", g.created);

    queries::run(&mut g, &facts).await;
    assert!(
        g.number_ok,
        "NUMBER-CHECK failed: a sequence number may have been consumed"
    );
}

/// Part 2 alone, GET only: the values come from existing objects (see
/// `existing.rs`), nothing is written, so it needs no budget and no sweep.
///
/// `just test-live --test request_gate case2`
#[tokio::test]
#[ignore = "reads the sevDesk test account"]
async fn case2_queries_read_only() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the read-only query gate");
        return;
    };
    let _alone = SERIAL.lock().await;
    let probe = Ctx::new(&token, "SDRS-GATE-READ-ONLY".to_owned());
    setup::verify_test_account(&probe).await;
    let (facts, marker) = existing::facts(&probe).await;
    // The `fulltext_search` queries look for the marker of the chosen invoice.
    let marker = marker.unwrap_or_else(|| {
        eprintln!("NOTE\tno SDRS-GATE- marker found on the chosen invoice");
        "SDRS-GATE-READ-ONLY".to_owned()
    });
    let ctx = Ctx::new(&token, marker);
    let inp = Inputs::read(&ctx).await;
    let mut g = Gate {
        ctx: &ctx,
        rec: Recorder::new(),
        inp,
        created: 0,
        sequences: BTreeMap::new(),
        number_ok: true,
    };
    g.rec.queries_to("queries_read_only.tsv");
    queries::run(&mut g, &facts).await;
    existing::by_name(&mut g).await;
    assert_eq!(g.created, 0, "the read-only gate wrote an object");
}
