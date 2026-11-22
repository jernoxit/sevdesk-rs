//! The length probe: how sevDesk handles LONG values in the position fields
//! `name` and `text` of invoice positions and credit note positions. Writes
//! PERMANENT documents to the sevDesk TEST account (the harness aborts on any
//! other account), so it is its own test target (`test = false` in Cargo.toml:
//! a plain `just test-live` skips it) and runs only on request:
//!
//! `just test-live --test length_probe <filter> -- --nocapture --test-threads=1`
//!
//! Every value is deterministic: the run marker, then repeated blocks
//! `[offset]„Tarif ä“ ` (offset = char index of the block start), cut at
//! exactly N chars, so a cut point is readable in what comes back. Lengths:
//! `control` 40, `realistic` 420, `oversize` 5000 chars.
//!
//! Each saved draft is one `LENGTHS\t…` line on stderr and one row of
//! `target/tmp/length_probe/<case>.tsv`. Columns: step, kind, field, level,
//! id, http, code, message, half_written, sent_chars, sent_bytes, got_chars,
//! got_bytes, equal, first_diff, tail. The position is re-read by its OWN GET
//! (`/InvoicePos?invoice[id]=…&invoice[objectName]=Invoice` resp.
//! `/CreditNotePos?creditNote[id]=…&creditNote[objectName]=CreditNote`), not
//! from the save answer. A save that did not succeed is followed by a
//! half-written check (by `customerInternalNote`).
//!
//! Cases:
//! - `case1_invoice_name`: one invoice draft per length, value in `name`.
//! - `case2_invoice_text`: the same, value in `text`, short `name`.
//! - `case3_credit_note_name`: credit note drafts (one shared origin invoice).
//! - `case4_credit_note_text`: the same with the value in `text`.
//! - `case5_credit_note_realistic_finalized`: one credit note draft with the
//!   `realistic` value in `name`, `sendBy` FINALIZE, then the e-invoice XML
//!   (does it carry the exact name) and a re-read of the position.
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

mod cases;
mod content;
mod probe;

use build::Kind;
use cases::Field;

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_invoice_name() {
    cases::lengths("case1_invoice_name", Kind::Invoice, Field::Name).await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case2_invoice_text() {
    cases::lengths("case2_invoice_text", Kind::Invoice, Field::Text).await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case3_credit_note_name() {
    cases::lengths("case3_credit_note_name", Kind::CreditNote, Field::Name).await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case4_credit_note_text() {
    cases::lengths("case4_credit_note_text", Kind::CreditNote, Field::Text).await;
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case5_credit_note_realistic_finalized() {
    cases::case5().await;
}
