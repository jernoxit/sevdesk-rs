//! Sample objects per resource, selected deterministically and scrubbed,
//! meant to become test fixtures.
use crate::scrub;
use crate::shape::form_pairs;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

type Pair = (String, &'static str);

const MAX_SAMPLES: usize = 4;

/// Key-name fragments (lower case) whose string values are blanked.
const BLANKED: [&str; 23] = [
    "iban",
    "bic",
    "pin",
    "password",
    "token",
    "apikey",
    "email",
    "phone",
    "mobile",
    "fax",
    "taxnumber",
    "vatnumber",
    "smtp",
    "lastloginip",
    "username",
    "firstname",
    "lastname",
    "fullname",
    "surename",
    "familyname",
    "birthday",
    "street",
    "name2",
];

/// Prefixes of values the probes wrote themselves; those may stay.
const PROBE_MARKERS: [&str; 2] = ["SDRS-", "PROBE"];

const POSITIONS: [&str; 3] = ["InvoicePos", "CreditNotePos", "VoucherPos"];

fn id_order(o: &Map<String, Value>) -> (i64, String) {
    match o.get("id") {
        Some(Value::Number(n)) => (n.as_i64().unwrap_or(i64::MAX), String::new()),
        Some(Value::String(s)) => (s.parse().unwrap_or(i64::MAX), s.clone()),
        _ => (i64::MAX, String::new()),
    }
}

fn non_null_keys(o: &Map<String, Value>) -> usize {
    o.values().filter(|v| !v.is_null()).count()
}

/// Indices into `pool`: first the object with the most non-null keys (tie:
/// lowest id), then greedily the object covering the most (key, form) pairs
/// not yet covered (tie: lowest id), until everything is covered or
/// `MAX_SAMPLES` are chosen.
pub(crate) fn select(pool: &[Map<String, Value>]) -> Vec<usize> {
    if pool.is_empty() {
        return Vec::new();
    }
    let pairs: Vec<BTreeSet<Pair>> = pool.iter().map(form_pairs).collect();
    let first = (0..pool.len())
        .min_by_key(|&i| {
            (
                std::cmp::Reverse(non_null_keys(&pool[i])),
                id_order(&pool[i]),
            )
        })
        .unwrap_or(0);
    let mut chosen = vec![first];
    let mut covered = pairs[first].clone();
    while chosen.len() < MAX_SAMPLES {
        let best = (0..pool.len())
            .filter(|i| !chosen.contains(i))
            .map(|i| (pairs[i].difference(&covered).count(), i))
            .filter(|(gain, _)| *gain > 0)
            .min_by_key(|&(gain, i)| (std::cmp::Reverse(gain), id_order(&pool[i])));
        let Some((_, i)) = best else { break };
        covered.extend(pairs[i].iter().cloned());
        chosen.push(i);
    }
    chosen
}

fn has_marker(s: &str) -> bool {
    PROBE_MARKERS.iter().any(|m| s.starts_with(m))
}

fn must_blank(resource: &str, key: &str) -> bool {
    let lower = key.to_lowercase();
    if lower == "addressname" || lower == "addressstreet" {
        return true;
    }
    if key == "name" && POSITIONS.contains(&resource) {
        return true;
    }
    BLANKED.iter().any(|f| lower.contains(f))
}

/// The same-form placeholder of a string: digits stay digits, empty stays
/// empty.
fn placeholder(s: &str) -> &'static str {
    if s.is_empty() {
        ""
    } else if s.bytes().all(|b| b.is_ascii_digit()) {
        "0000"
    } else {
        "REDACTED"
    }
}

/// Blanks the string values of personal or secret keys, at any depth. `null`,
/// numbers and booleans stay; a probe-marked `name` / `addressName` /
/// `addressStreet` stays.
pub(crate) fn redact(resource: &str, v: &mut Value) {
    match v {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                if let Value::String(s) = value {
                    let probe_exempt =
                        matches!(key.to_lowercase().as_str(), "addressname" | "addressstreet")
                            || (key == "name" && POSITIONS.contains(&resource));
                    let keep = probe_exempt && has_marker(s);
                    if must_blank(resource, key) && !keep {
                        *value = Value::String(placeholder(s).to_owned());
                    }
                } else {
                    redact(resource, value);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|x| redact(resource, x)),
        _ => {}
    }
}

fn id_text(o: &Map<String, Value>) -> String {
    match o.get("id") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "?".into(),
    }
}

/// The coverage sidecar: every (key, form) pair of the pool with the sample
/// ids covering it, or `uncovered`.
pub(crate) fn coverage_tsv(pool: &[Map<String, Value>], chosen: &[usize]) -> String {
    let all: BTreeSet<Pair> = pool.iter().flat_map(form_pairs).collect();
    let per_sample: Vec<(String, BTreeSet<Pair>)> = chosen
        .iter()
        .map(|&i| (id_text(&pool[i]), form_pairs(&pool[i])))
        .collect();
    let mut out = String::from("key\tform\tstatus\tsamples\n");
    for pair in &all {
        let ids: Vec<&str> = per_sample
            .iter()
            .filter(|(_, p)| p.contains(pair))
            .map(|(id, _)| id.as_str())
            .collect();
        let status = if ids.is_empty() {
            "uncovered"
        } else {
            "covered"
        };
        out.push_str(&format!(
            "{}\t{}\t{status}\t{}\n",
            pair.0,
            pair.1,
            ids.join(",")
        ));
    }
    out
}

/// Writes `samples/<resource>.json` and `.coverage.tsv` below `dir`. The
/// answer was collapsed by the caller; the envelope also passes the shared
/// capture scrubber, then the redaction above.
pub(crate) fn write_samples(dir: &str, resource: &str, pool: &[Map<String, Value>]) {
    let chosen = select(pool);
    if chosen.is_empty() {
        return;
    }
    let objects: Vec<Value> = chosen
        .iter()
        .map(|&i| Value::Object(pool[i].clone()))
        .collect();
    let envelope = serde_json::json!({ "objects": objects }).to_string();
    let mut v: Value = serde_json::from_str(&scrub::scrub_capture(&envelope)).unwrap();
    redact(resource, &mut v);
    let out = format!("{dir}/samples");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        format!("{out}/{resource}.json"),
        serde_json::to_string_pretty(&v).unwrap(),
    )
    .unwrap();
    std::fs::write(
        format!("{out}/{resource}.coverage.tsv"),
        coverage_tsv(pool, &chosen),
    )
    .unwrap();
    eprintln!("SURVEY-SAMPLES\t{resource}\t{}", chosen.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn greedy_selection_covers_new_pairs_and_breaks_ties_by_id() {
        let pool = vec![
            obj(json!({"id": "5", "a": 1, "b": 2, "c": null})),
            obj(json!({"id": "3", "a": 1, "b": 2, "c": null})),
            obj(json!({"id": "9", "a": 1, "b": "x", "c": null})),
            obj(json!({"id": "7", "a": 1, "b": 2, "c": "2020-01-01"})),
            obj(json!({"id": "8", "a": 1, "b": 2, "c": "2020-01-02"})),
        ];
        let ids: Vec<String> = select(&pool).iter().map(|&i| id_text(&pool[i])).collect();
        // most non-null keys, lowest id among the 4-key objects: 7; then 9
        // (two new pairs: c null, b str_other) beats 3 and 5 (one each)
        assert_eq!(ids, ["7", "9"]);
        let tsv = coverage_tsv(&pool, &select(&pool));
        assert!(tsv.contains("c\tnull\tcovered\t9"));
        assert!(tsv.contains("b\tint\tcovered\t7"));
        assert!(!tsv.contains("uncovered"));
        assert!(tsv.contains("b\tstr_other\tcovered\t9"));
    }

    #[test]
    fn redaction_keeps_form_and_probe_markers() {
        let mut v = json!({"objects": [{
            "email": "a@b.de", "iban": "DE001234", "phone": "0123", "fax": "",
            "lastName": null, "street": "Main 1", "pin": 7,
            "name": "SDRS-pos", "addressName": "Real Name",
            "addressStreet": "PROBE street", "note": "keep",
            "contact": {"surename": "Doe"}
        }]});
        redact("InvoicePos", &mut v);
        let o = &v["objects"][0];
        assert_eq!(o["email"], "REDACTED");
        assert_eq!(o["iban"], "REDACTED");
        assert_eq!(o["phone"], "0000");
        assert_eq!(o["fax"], "");
        assert!(o["lastName"].is_null());
        assert_eq!(o["street"], "REDACTED");
        assert_eq!(o["pin"], 7);
        assert_eq!(o["name"], "SDRS-pos");
        assert_eq!(o["addressName"], "REDACTED");
        assert_eq!(o["addressStreet"], "PROBE street");
        assert_eq!(o["note"], "keep");
        assert_eq!(o["contact"]["surename"], "REDACTED");
        let mut other = json!({"name": "Real"});
        redact("Contact", &mut other);
        assert_eq!(other["name"], "Real");
        let mut pos = json!({"name": "Real"});
        redact("VoucherPos", &mut pos);
        assert_eq!(pos["name"], "REDACTED");
    }
}
