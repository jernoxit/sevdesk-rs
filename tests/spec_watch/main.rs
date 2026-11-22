//! Spec watcher: hints about what a newly published sevDesk OpenAPI spec adds
//! relative to the crate's read types and endpoints. It generates nothing.
//!
//! Run with `just spec-check`, which downloads the spec to
//! `target/tmp/spec/openapi.yaml` (public, no token). The spec is not
//! committed (no license); only its checksum is. The target is `test = false`,
//! so `just test` and CI never need the download.
//!
//! Seven checks, each against a named committed list by EXACT equality: growth
//! and shrinkage both fail, and the message prints the new list to paste.
//!
//! 1. the spec's SHA-256 ([`KNOWN_SPEC_SHA256`]);
//! 2. spec properties a read type does not read ([`fields::SPEC_FIELDS_NOT_READ`]);
//! 3. spec enum values that land in a read enum's `Other` arm
//!    ([`enums::SPEC_ENUM_VALUES_AS_OTHER`]);
//! 4. spec endpoints ([`endpoints::IMPLEMENTED`], [`endpoints::NOT_IMPLEMENTED`]);
//! 5. writable spec properties of a request that no request type sends
//!    ([`request_fields::SPEC_WRITABLE_NOT_SENT`]);
//! 6. keys a request type sends that the spec does not list as writable
//!    ([`request_fields::SENT_NOT_IN_SPEC`]);
//! 7. list filters of the spec the query types do not send
//!    ([`filters::SPEC_FILTERS_NOT_IN_API`]), and filters they send that the
//!    spec does not list ([`filters::SENT_FILTERS_NOT_IN_SPEC`]).
//!
//! The other checks run even when the checksum differs.

mod endpoints;
mod enums;
mod fields;
mod filters;
mod request_fields;
mod request_samples;

use serde_yaml_ng::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// The one place the spec is read from; no environment variable.
const SPEC_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/tmp/spec/openapi.yaml");

/// SHA-256 of the spec the lists below were filled from.
const KNOWN_SPEC_SHA256: &str = "2f4931a876c0c3fad778382ad2ae6f3ed1b3545dc0542a75cd214bf6e7c649e0";

/// Compares `found` with `listed` by exact equality. `None` when equal;
/// otherwise a message naming what is new, what is gone, and the new list.
fn exact(name: &str, found: &BTreeSet<String>, listed: &[(&str, &str)]) -> Option<String> {
    exact_with_hint(name, found, listed, "")
}

/// Like [`exact`], with a `hint` line on what to do about a NEW entry.
fn exact_with_hint(
    name: &str,
    found: &BTreeSet<String>,
    listed: &[(&str, &str)],
    hint: &str,
) -> Option<String> {
    let listed_keys: BTreeSet<String> = listed.iter().map(|(k, _)| (*k).to_owned()).collect();
    if *found == listed_keys {
        return None;
    }
    let mut msg = format!("{name} differs from the spec or the crate:\n");
    for key in found.difference(&listed_keys) {
        msg.push_str(&format!("  NEW, not listed: {key}\n"));
    }
    if !hint.is_empty() && found.difference(&listed_keys).next().is_some() {
        msg.push_str(&format!("  hint: {hint}\n"));
    }
    for key in listed_keys.difference(found) {
        msg.push_str(&format!("  LISTED, no longer found: {key}\n"));
    }
    msg.push_str(&format!("new {name} to paste:\n"));
    for key in found {
        let reason = listed
            .iter()
            .find(|(k, _)| k == key)
            .map_or("TODO: reason", |(_, r)| *r);
        msg.push_str(&format!("    ({key:?}, {reason:?}),\n"));
    }
    Some(msg)
}

/// The property names of a spec schema.
fn schema_properties(spec: &Value, schema: &str) -> Vec<String> {
    spec.get("components")
        .and_then(|v| v.get("schemas"))
        .and_then(|v| v.get(schema))
        .and_then(|v| v.get("properties"))
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("spec has no schema {schema} with properties"))
        .keys()
        .filter_map(|k| k.as_str().map(str::to_owned))
        .collect()
}

/// The writable properties (no `readOnly: true`) of a request schema: a name
/// in `components.schemas`, or `METHOD path` for an inline request body.
fn writable_properties(spec: &Value, label: &str) -> BTreeSet<String> {
    let schema = match label.split_once(' ') {
        None => spec
            .get("components")
            .and_then(|v| v.get("schemas"))
            .and_then(|v| v.get(label)),
        Some((method, path)) => spec
            .get("paths")
            .and_then(|v| v.get(path))
            .and_then(|v| v.get(method.to_lowercase()))
            .and_then(|v| v.get("requestBody"))
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("application/json"))
            .and_then(|v| v.get("schema")),
    };
    schema
        .and_then(|v| v.get("properties"))
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("spec has no request schema {label} with properties"))
        .iter()
        .filter(|(_, v)| v.get("readOnly").and_then(Value::as_bool) != Some(true))
        .filter_map(|(k, _)| k.as_str().map(str::to_owned))
        .collect()
}

/// The enum values of one spec property (`null` entries are skipped).
fn property_enum(spec: &Value, schema: &str, property: &str) -> Vec<String> {
    spec.get("components")
        .and_then(|v| v.get("schemas"))
        .and_then(|v| v.get(schema))
        .and_then(|v| v.get("properties"))
        .and_then(|v| v.get(property))
        .and_then(|v| v.get("enum"))
        .and_then(Value::as_sequence)
        .unwrap_or_else(|| panic!("spec has no enum at {schema}.{property}"))
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .collect()
}

fn checksum_problem(bytes: &[u8]) -> Option<String> {
    let digest = Sha256::digest(bytes);
    let actual: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    (actual != KNOWN_SPEC_SHA256).then(|| {
        format!(
            "new spec version: SHA-256 is {actual}, KNOWN_SPEC_SHA256 is {KNOWN_SPEC_SHA256}; \
             review the findings below, then set the constant"
        )
    })
}

#[test]
#[ignore = "needs target/tmp/spec/openapi.yaml from just spec-check"]
fn spec_matches_our_types() {
    let bytes = std::fs::read(SPEC_PATH).unwrap_or_else(|e| panic!("{SPEC_PATH}: {e}"));
    let spec: Value =
        serde_yaml_ng::from_slice(&bytes).unwrap_or_else(|e| panic!("{SPEC_PATH}: {e}"));

    let mut problems: Vec<String> = Vec::new();
    problems.extend(checksum_problem(&bytes));
    problems.extend(fields::problems(&spec));
    problems.extend(enums::problems(&spec));
    problems.extend(endpoints::problems(&spec));
    problems.extend(request_fields::problems(&spec));
    problems.extend(filters::problems(&spec));

    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
