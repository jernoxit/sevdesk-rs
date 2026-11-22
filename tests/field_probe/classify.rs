//! Compares what was sent with what the object reads back and names one of
//! `stored`, `ignored`, `refused`, `changed-other` (plus `same-value`: the sent
//! value equals the value before, so nothing can be told).

use crate::table::Kind;
use serde_json::Value;
use time::OffsetDateTime;

/// Keys that change on every write and say nothing.
const NOISE: [&str; 1] = ["update"];

pub(crate) fn id_i64(v: &Value) -> Option<i64> {
    match &v["id"] {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_i64(),
        _ => None,
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_owned(),
        other => other.to_string(),
    }
}

/// Decimal text without trailing zeros (`"2.50"` and `2.5` agree).
fn decimal(v: &Value) -> String {
    let s = text(v);
    if s.contains('.')
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

fn flag(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) if s == "1" || s == "true" => Some(true),
        Value::String(s) if s == "0" || s == "false" => Some(false),
        Value::Number(n) if n.as_i64() == Some(1) => Some(true),
        Value::Number(n) if n.as_i64() == Some(0) => Some(false),
        _ => None,
    }
}

/// The calendar day (`yyyy-mm-dd`) of a date in any of the probed encodings.
pub(crate) fn day(v: &Value) -> Option<String> {
    let unix = |secs: i64| {
        OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(|t| t.date().to_string())
    };
    match v {
        Value::Number(n) => n.as_i64().and_then(unix),
        Value::String(s) => {
            let b = s.as_bytes();
            if b.len() >= 10 && b[4] == b'-' && b[7] == b'-' {
                s.get(..10).map(str::to_owned)
            } else if b.len() == 10 && b[2] == b'.' && b[5] == b'.' {
                Some(format!(
                    "{}-{}-{}",
                    s.get(6..10)?,
                    s.get(3..5)?,
                    s.get(0..2)?
                ))
            } else if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                s.parse().ok().and_then(unix)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn equivalent(kind: Kind, sent: &Value, got: Option<&Value>) -> bool {
    let Some(got) = got.filter(|g| !g.is_null()) else {
        return false;
    };
    match kind {
        Kind::Str => text(sent) == text(got),
        Kind::Num => decimal(sent) == decimal(got),
        Kind::Bool => flag(sent).is_some() && flag(sent) == flag(got),
        Kind::Date => day(sent).is_some() && day(sent) == day(got),
        Kind::Ref => id_i64(sent).is_some() && id_i64(sent) == id_i64(got),
    }
}

/// Keys other than `field` whose value differs between the two reads.
pub(crate) fn changed_keys(field: &str, before: &Value, after: &Value) -> Vec<String> {
    let (Some(b), Some(a)) = (before.as_object(), after.as_object()) else {
        return Vec::new();
    };
    let mut keys: Vec<&String> = b.keys().chain(a.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| *k != field && !NOISE.contains(&k.as_str()) && b.get(*k) != a.get(*k))
        .cloned()
        .collect()
}

/// The class and a note. `before` and `after` are the head (or position) objects
/// around the update; `failed` is HTTP 300 or more or an `error` in the body.
pub(crate) fn classify(
    kind: Kind,
    field: &str,
    sent: &Value,
    before: &Value,
    after: &Value,
    failed: bool,
) -> (&'static str, String) {
    let others = changed_keys(field, before, after);
    let own_changed = before.get(field) != after.get(field);
    let tail = if others.is_empty() {
        String::new()
    } else {
        format!("; also changed: {}", others.join(","))
    };
    if failed {
        let anyway = if own_changed || !others.is_empty() {
            format!(
                "re-read: changed anyway (field {own_changed}{})",
                if others.is_empty() {
                    String::new()
                } else {
                    format!(", others {}", others.join(","))
                }
            )
        } else {
            "re-read: nothing changed".to_owned()
        };
        return ("refused", anyway);
    }
    if equivalent(kind, sent, after.get(field)) {
        if equivalent(kind, sent, before.get(field)) {
            (
                "same-value",
                format!("sent value equals the value before{tail}"),
            )
        } else {
            ("stored", format!("read back equivalent{tail}"))
        }
    } else if own_changed {
        (
            "changed-other",
            format!("the field itself read back differently{tail}"),
        )
    } else if !others.is_empty() {
        ("changed-other", format!("field unchanged{tail}"))
    } else {
        ("ignored", "nothing changed".to_owned())
    }
}
