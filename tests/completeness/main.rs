//! Completeness guard: every key a recorded answer carries is either read by
//! the read type or stands on that fixture's named list of unread keys.
//!
//! No network: each fixture is decoded into its read type the way the client
//! decodes `objects`, collecting every key the type ignores (`serde_ignored`).
//! The lists are compared by EXACT equality: a key that appears (the type lost
//! a field, the fixture changed) fails, and a key that disappears (a field was
//! added) fails too, with the new list in the message so it is lowered in the
//! same commit. List indices are normalized to `*`.

#[path = "../common/mod.rs"]
mod common;

mod check_account;
mod contact;
mod credit_note;
mod invoice;
mod misc;
mod transaction;
mod voucher;

use common::fixture;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;
use std::collections::BTreeSet;

/// One covered fixture: its name, the keys its read type ignores (found), and
/// the keys that are listed as unread.
type Covered = (&'static str, Vec<String>, &'static [&'static str]);

/// The keys of `name` that `T` (or `Vec<T>` for a list answer) does not read.
fn unread<T: DeserializeOwned>(name: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct Envelope {
        objects: Box<RawValue>,
    }
    let envelope: Envelope =
        serde_json::from_str(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let raw = envelope.objects.get();
    let is_list = raw.trim_start().starts_with('[');
    let mut found = BTreeSet::new();
    let mut de = serde_json::Deserializer::from_str(raw);
    let record = |path: serde_ignored::Path<'_>| {
        let segments: Vec<String> = path
            .to_string()
            .split('.')
            .filter(|s| *s != "?")
            .map(|s| {
                if s.bytes().all(|b| b.is_ascii_digit()) {
                    "*".to_owned()
                } else {
                    s.to_owned()
                }
            })
            .collect();
        found.insert(format!("objects.{}", segments.join(".")));
    };
    if is_list {
        serde_ignored::deserialize::<_, _, Vec<T>>(&mut de, record)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    } else {
        serde_ignored::deserialize::<_, _, T>(&mut de, record)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    let embedded = invoice::embedded_objects(name);
    found
        .into_iter()
        .map(
            |key| match embedded.iter().find(|p| key.starts_with(&format!("{p}."))) {
                Some(prefix) => format!("{prefix}.<embedded object>"),
                None => key,
            },
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Full paths of the form-unmeasured keys of `prefix` + each key of `keys`.
fn under(prefix: &str, keys: &[(&str, bool)]) -> Vec<(String, bool)> {
    keys.iter()
        .map(|(k, recorded)| (format!("{prefix}{k}"), *recorded))
        .collect()
}

/// Fixtures whose read type has a form-unmeasured list: the full path of each
/// key of that list in the fixture.
fn form_unmeasured(fixture: &str) -> Vec<(String, bool)> {
    invoice::form_unmeasured(fixture)
        .or_else(|| credit_note::form_unmeasured(fixture))
        .unwrap_or_default()
}

/// Fixtures that are read answers, with the read type they decode into and
/// the keys that type does not read yet.
fn covered() -> Vec<Covered> {
    let mut all = Vec::new();
    all.extend(check_account::covered());
    all.extend(contact::covered());
    all.extend(credit_note::covered());
    all.extend(invoice::covered());
    all.extend(misc::covered());
    all.extend(transaction::covered());
    all.extend(voucher::covered());
    all
}

/// Fixtures that are deliberately not decoded here, with the reason.
const SKIPPED: &[(&str, &str)] = &[
    ("balance_at_date", "a bare amount string, no object"),
    (
        "bookkeeping_system_version",
        "read by a private one-field struct",
    ),
    ("error_404_code_151", "an error body"),
    ("error_422_offline_account", "an error body"),
    ("invoice_pdf", "read into a private wire type (DocumentPdf)"),
    ("invoice_xml", "a bare XML string, no object"),
    (
        "write_transaction_delete",
        "answer is a list holding null, no read type",
    ),
    (
        "write_voucher_book_amount",
        "write answer the client discards, no read type",
    ),
    (
        "write_voucher_reset_to_open",
        "write answer the client discards, no read type",
    ),
];

#[test]
fn unread_keys_equal_the_named_lists() {
    let mut failures = Vec::new();
    for (name, found, listed) in covered() {
        let mut expected: Vec<String> = listed.iter().map(|k| (*k).to_owned()).collect();
        // A form-unmeasured key is expected exactly where the fixture has it.
        expected.extend(
            form_unmeasured(name)
                .into_iter()
                .map(|(key, _)| key)
                .filter(|key| found.contains(key)),
        );
        expected.sort();
        if found != expected {
            failures.push(format!(
                "{name}: unread keys changed ({} -> {}); new list:\n{found:#?}",
                expected.len(),
                found.len()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A key on a form-unmeasured list must still be unread: when a field is added
/// for it, it leaves the list in the same commit.
#[test]
fn form_unmeasured_keys_are_still_unread_in_some_fixture() {
    let all = covered();
    let bare = |path: &str| path.rsplit('.').next().unwrap_or(path).to_owned();
    let mut stale = BTreeSet::new();
    for (name, _, _) in &all {
        for (key, recorded) in form_unmeasured(name) {
            let shown = all.iter().any(|(other, found, _)| {
                form_unmeasured(other)
                    .iter()
                    .any(|(k, _)| bare(k) == bare(&key))
                    && found.iter().any(|f| bare(f) == bare(&key))
            });
            if recorded && !shown {
                stale.insert(bare(&key));
            }
        }
    }
    assert!(stale.is_empty(), "no fixture shows these unread: {stale:?}");
}

#[test]
fn every_fixture_is_covered_or_skipped() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let on_disk: BTreeSet<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".json").map(str::to_owned))
        .collect();
    let known: BTreeSet<String> = covered()
        .into_iter()
        .map(|(n, ..)| n.to_owned())
        .chain(SKIPPED.iter().map(|(n, _)| (*n).to_owned()))
        .collect();
    assert_eq!(
        on_disk, known,
        "fixtures and the covered/skipped lists differ"
    );
}
