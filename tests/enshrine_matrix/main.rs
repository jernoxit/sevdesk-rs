//! The enshrine matrix: what a finalized (status 200) and an enshrined
//! (`enshrined` set) invoice or credit note allow. Writes PERMANENT documents
//! to the sevDesk TEST account, so it is its own test target (`test = false`
//! in Cargo.toml: a plain `just test-live` skips it) and runs only on request:
//!
//! `just test-live --test enshrine_matrix run1 -- --nocapture`
//!
//! `run1_setting_on` is meant for the account setting "Automatische
//! Festschreibung von Dokumenten bei Fertigstellung" ON, `run2_setting_off`
//! for OFF; the test cannot read the setting and does not touch it. Every
//! measured step is one `MATRIX\t…` line on stderr and one row of
//! `target/tmp/enshrine_matrix/ensh-<run>.tsv`.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness files are shared with live_write and use only some of the helpers; a failure in a test helper is a test failure"
)]

#[path = "../live_write/docs.rs"]
mod docs;
#[path = "../live_write/harness.rs"]
mod harness;
#[path = "../common/scrub.rs"]
mod scrub;

mod build;
mod matrix;
mod ops;
mod record;

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn run1_setting_on() {
    matrix::run("run1").await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn run2_setting_off() {
    matrix::run("run2").await;
}
