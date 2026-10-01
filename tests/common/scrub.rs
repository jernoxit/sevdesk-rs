//! Keeps account and personal data out of recorded fixtures: sevDesk expands
//! `SevClient` / `SevUser` references into the full object (company, tax
//! number, bank data, last login ...); a fixture keeps only the reference.

use serde_json::{Map, Value, json};

/// Collapses every expanded `SevClient` / `SevUser` object, at any depth, to
/// `{"id", "objectName"}`. Returns how many were collapsed.
pub fn collapse_account_objects(v: &mut Value) -> usize {
    match v {
        Value::Object(map) => {
            if is_expanded(map) {
                *v = json!({ "id": map["id"], "objectName": map["objectName"] });
                return 1;
            }
            map.values_mut().map(collapse_account_objects).sum()
        }
        Value::Array(items) => items.iter_mut().map(collapse_account_objects).sum(),
        _ => 0,
    }
}

fn is_expanded(map: &Map<String, Value>) -> bool {
    matches!(
        map.get("objectName").and_then(Value::as_str),
        Some("SevClient" | "SevUser")
    ) && map.contains_key("id")
        && map.len() > 2
}

/// Scrubs a recorded response body. A body that is not JSON is returned as is.
pub fn scrub_capture(body: &str) -> String {
    let Ok(mut v) = serde_json::from_str::<Value>(body) else {
        return body.to_owned();
    };
    if collapse_account_objects(&mut v) > 0 {
        v.to_string()
    } else {
        body.to_owned()
    }
}
