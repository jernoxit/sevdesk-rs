//! The address probe: does sevDesk keep a document's individual address fields
//! (`addressName`, `addressStreet`, `addressZip`, `addressCity`,
//! `addressCountry`) after the save, or does it move them into the free-text
//! field `address`? Writes PERMANENT documents to the sevDesk TEST account (the
//! harness aborts on any other account), so it is its own test target
//! (`test = false` in Cargo.toml: a plain `just test-live` skips it) and runs
//! only on request:
//!
//! `just test-live --test address_probe case1`
//!
//! Four drafts, all five address fields set, the free-text `address` NOT sent
//! (the payload leaves it out): an invoice and a credit note with a plain
//! `addressName`, the same two with two leading and two trailing spaces in it.
//! Each draft is read three times: the save answer, `GET /Invoice/{id}` resp.
//! `GET /CreditNote/{id}`, and the list `GET /Invoice?customerInternalNote=…`
//! resp. `GET /CreditNote?customerInternalNote=…`. The comparison with what
//! was sent is exact string equality, no trimming.
//!
//! Each read is one `ADDRESS\t…` line on stderr and one row of
//! `target/tmp/address_probe/case1_address_fields_after_save.tsv`. Columns:
//! step, kind, id, read, http, name, street, zip, city, country_id, address,
//! all_equal, differing_fields. Values are JSON-encoded, so spaces and `null`
//! are visible.
//!
//! Cases:
//! - `case1_address_fields_after_save`: the four drafts above (one shared
//!   origin invoice for the credit notes).
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

mod probe;

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_address_fields_after_save() {
    probe::case1("case1_address_fields_after_save").await;
}
