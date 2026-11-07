//! The number probe: how sevDesk handles DUPLICATE document numbers of credit
//! notes and invoices, one after the other and at the same time. Writes
//! PERMANENT documents to the sevDesk TEST account (the harness aborts on any
//! other account), so it is its own test target (`test = false` in Cargo.toml:
//! a plain `just test-live` skips it) and runs only on request:
//!
//! `just test-live --test number_probe <filter> -- --nocapture --test-threads=1`
//!
//! Cases (each step is one `NUMBERS\t…` line on stderr and one row of
//! `target/tmp/number_probe/<case>.tsv`; columns: step, kind, id, HTTP status,
//! sevDesk code, message, number in the answer, number re-read by id, the
//! number sequence before and after, wall-clock start/end in ms):
//!
//! - `case1_credit_notes_sequential`: two drafts with the same credit note
//!   number; is the second create refused; what do both end with after
//!   finalize and enshrine.
//! - `case2_credit_notes_concurrent`: the same with two clients, creates,
//!   finalizes and enshrines running at the same time (overlap is visible in
//!   the timestamps).
//! - `case3_credit_note_reuses_enshrined_number`: a new draft takes the
//!   number of an already enshrined credit note.
//! - `case4_invoices_duplicate_number`: invoices; the draft has no number, so
//!   `invoiceNumber` is set by a raw `PUT /Invoice/{id}`; sequential (a) and
//!   concurrent (b).
//! - `case5_credit_note_number_by_sevdesk`: can sevDesk assign the credit note
//!   number: (a) a placeholder number, (b) `createFromInvoice`. Each is
//!   followed by a half-written-document check.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness files are shared with live_write and use only some of the helpers; a failure in a test helper is a test failure"
)]

#[path = "../enshrine_matrix/build.rs"]
mod build;
#[path = "../live_write/docs.rs"]
mod docs;
#[path = "../live_write/harness.rs"]
mod harness;
#[path = "../enshrine_matrix/ops.rs"]
mod ops;
#[path = "../enshrine_matrix/record.rs"]
mod record;
#[path = "../common/scrub.rs"]
mod scrub;

mod calls;
mod cases_credit_note;
mod cases_invoice;
mod probe;

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_credit_notes_sequential() {
    cases_credit_note::case1().await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case2_credit_notes_concurrent() {
    cases_credit_note::case2().await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case3_credit_note_reuses_enshrined_number() {
    cases_credit_note::case3().await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case4_invoices_duplicate_number() {
    cases_invoice::case4().await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case5_credit_note_number_by_sevdesk() {
    cases_credit_note::case5().await;
}
