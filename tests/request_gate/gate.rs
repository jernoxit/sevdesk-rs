//! The recorder (TSV files and stderr lines) and the comparison of a body the
//! crate sent with the raw object sevDesk reads back.
//!
//! Classes of a field: `stored` (reads back equal), `ignored` (reads back
//! null/absent, or unchanged by the write), `changed` (reads back different:
//! sent and read-back value are both recorded), `refused` (the call failed;
//! the re-read tells whether it was written anyway).

use serde_json::Value;
use std::collections::BTreeMap;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Class {
    Stored,
    Ignored,
    Changed,
    Refused,
}

impl Class {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::Ignored => "ignored",
            Self::Changed => "changed",
            Self::Refused => "refused",
        }
    }
}

/// What a list query is expected to do with the unfiltered count.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Expect {
    /// Non-empty and smaller than the unfiltered list.
    Narrows,
    /// Non-empty.
    NonEmpty,
    /// The same set as the unfiltered list (a sort).
    Same,
    /// Larger than the unfiltered list (a mode that widens).
    Wider,
}

struct Row {
    route: String,
    field: String,
    class: Class,
    sent: String,
    got: String,
    note: String,
}

pub(crate) struct Recorder {
    dir: String,
    rows: BTreeMap<String, Vec<Row>>,
    queries: Vec<String>,
}

fn clean(s: &str) -> String {
    let flat = s.replace(['\t', '\r'], " ").replace('\n', "\\n");
    flat.chars().take(200).collect()
}

impl Recorder {
    pub(crate) fn new() -> Self {
        let dir = format!("{}/request_gate", env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(&dir).unwrap();
        Self {
            dir,
            rows: BTreeMap::new(),
            queries: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn row(
        &mut self,
        object: &str,
        route: &str,
        field: &str,
        class: Class,
        sent: &Value,
        got: &Value,
        note: &str,
    ) {
        eprintln!("GATE\t{object}\t{route}\t{field}\t{}", class.as_str());
        let text = |v: &Value| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        self.rows.entry(object.to_owned()).or_default().push(Row {
            route: route.to_owned(),
            field: field.to_owned(),
            class,
            sent: clean(&text(sent)),
            got: clean(&text(got)),
            note: clean(note),
        });
    }

    /// A free-form measurement row (no comparison), e.g. the voucher gross case.
    pub(crate) fn note(&mut self, object: &str, route: &str, field: &str, note: &str) {
        eprintln!("GATE\t{object}\t{route}\t{field}\tnote\t{note}");
        self.rows.entry(object.to_owned()).or_default().push(Row {
            route: route.to_owned(),
            field: field.to_owned(),
            class: Class::Stored,
            sent: String::new(),
            got: String::new(),
            note: format!("NOTE {}", clean(note)),
        });
    }

    /// Writes `<object>.tsv`; called after every object so a later panic keeps it.
    pub(crate) fn flush(&self, object: &str) {
        let mut out = String::from("route\tfield\tclass\tsent\tgot\tnote\n");
        for r in self.rows.get(object).map(Vec::as_slice).unwrap_or_default() {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                r.route,
                r.field,
                r.class.as_str(),
                r.sent,
                r.got,
                r.note
            ));
        }
        std::fs::write(format!("{}/{object}.tsv", self.dir), out).unwrap();
    }

    /// One query parameter: `result` is the length of the answer or the error.
    pub(crate) fn query(
        &mut self,
        query: &str,
        param: &str,
        expect: Expect,
        unfiltered: usize,
        result: Result<usize, impl std::fmt::Display>,
        note: &str,
    ) {
        let (class, n, note) = match result {
            Err(e) => ("error", 0, clean(&e.to_string())),
            Ok(n) => {
                let class = match expect {
                    _ if n == 0 => "empty",
                    Expect::Narrows if n >= unfiltered => "not-narrowed",
                    Expect::Same if n != unfiltered => "not-narrowed",
                    Expect::Wider if n <= unfiltered => "not-narrowed",
                    _ => "ok",
                };
                (class, n, clean(note))
            }
        };
        eprintln!("GATEQ\t{query}\t{param}\t{class}\t{n}/{unfiltered}");
        self.queries.push(format!(
            "{query}\t{param}\t{class}\t{n}\t{unfiltered}\t{expect:?}\t{note}"
        ));
        self.flush_queries();
    }

    /// A parameter that could not be tried for lack of live data.
    pub(crate) fn query_skipped(&mut self, query: &str, param: &str, unfiltered: usize, why: &str) {
        eprintln!("GATEQ\t{query}\t{param}\tno-data\t0/{unfiltered}");
        self.queries.push(format!(
            "{query}\t{param}\tno-data\t0\t{unfiltered}\t-\t{}",
            clean(why)
        ));
        self.flush_queries();
    }

    fn flush_queries(&self) {
        let mut out = String::from("query\tparam\tclass\tn\tunfiltered\texpected\tnote\n");
        for q in &self.queries {
            out.push_str(q);
            out.push('\n');
        }
        std::fs::write(format!("{}/queries.tsv", self.dir), out).unwrap();
    }
}

/// The `id` of a `{id, objectName}` reference or of an object, as a number.
pub(crate) fn id_of(v: &Value) -> Option<i64> {
    match &v["id"] {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_i64(),
        _ => None,
    }
}

/// Keys that are not fields of the object.
const SKIP: [&str; 3] = ["objectName", "mapAll", "invoicePosDelete"];

/// Compares every field of `sent` (the crate's own body of one object) with
/// the raw object read back. `before` is the object before an update.
pub(crate) fn compare(
    rec: &mut Recorder,
    object: &str,
    route: &str,
    sent: &Value,
    before: Option<&Value>,
    got: &Value,
    failed: bool,
) {
    let Some(map) = sent.as_object() else {
        return;
    };
    for (key, s) in map {
        if SKIP.contains(&key.as_str()) || s.is_null() || s.is_array() {
            continue;
        }
        let g = got.get(key).filter(|g| !g.is_null());
        let b = before.and_then(|b| b.get(key)).filter(|b| !b.is_null());
        let (class, note) = judge(key, s, b, g, failed);
        rec.row(
            object,
            route,
            key,
            class,
            s,
            g.unwrap_or(&Value::Null),
            &note,
        );
    }
}

/// Panics before the write when the crate's body lacks a field that the
/// builder meant to set (an option left out by mistake). Keys that depend on a
/// live lookup that found nothing are reported as untested instead.
pub(crate) fn expect_keys(what: &str, sent: &Value, expected: &[&str], lookup_dependent: &[&str]) {
    for key in expected {
        if sent.get(key).is_some() {
            continue;
        }
        if lookup_dependent.contains(key) {
            eprintln!("UNTESTED\t{what}\t{key}\tno live value to send");
        } else {
            panic!("{what}: the crate's body has no `{key}`: the builder forgot an option");
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Eq {
    Yes,
    /// Same calendar day, different instant.
    Day,
    No,
}

fn judge(
    key: &str,
    sent: &Value,
    before: Option<&Value>,
    got: Option<&Value>,
    failed: bool,
) -> (Class, String) {
    let eq = |v: &Value| equivalence(key, sent, v);
    if failed {
        let written =
            got.is_some_and(|g| eq(g) == Eq::Yes) && !before.is_some_and(|b| eq(b) == Eq::Yes);
        let note = if written {
            "re-read: WRITTEN anyway"
        } else {
            "re-read: not written"
        };
        return (Class::Refused, note.to_owned());
    }
    let Some(got) = got else {
        return (Class::Ignored, "read-back is null or absent".to_owned());
    };
    match eq(got) {
        Eq::Yes if before.is_some_and(|b| eq(b) == Eq::Yes) => {
            (Class::Stored, "same as before the write".to_owned())
        }
        Eq::Yes => (Class::Stored, String::new()),
        _ if before == Some(got) => (Class::Ignored, "unchanged by the write".to_owned()),
        Eq::Day => (Class::Changed, "only the calendar day equals".to_owned()),
        _ => (Class::Changed, "the value differs".to_owned()),
    }
}

fn yes_if(b: bool) -> Eq {
    if b { Eq::Yes } else { Eq::No }
}

fn is_date_key(key: &str) -> bool {
    key.ends_with("Date") || key.ends_with("DateUntil")
}

fn equivalence(key: &str, sent: &Value, got: &Value) -> Eq {
    match sent {
        Value::Object(_) => yes_if(id_of(sent).is_some() && id_of(sent) == id_of(got)),
        Value::Bool(b) => yes_if(flag(got) == Some(*b)),
        Value::Number(n) if n.is_i64() && is_date_key(key) => {
            yes_if(instant(got).is_some() && instant(got) == n.as_i64())
        }
        Value::String(s) => {
            if let Some(day) = sent_day(s) {
                yes_if(day_of(got).as_deref() == Some(day.as_str()))
            } else if let Ok(t) = OffsetDateTime::parse(s, &Rfc3339) {
                if instant(got) == Some(t.unix_timestamp()) {
                    Eq::Yes
                } else if day_of(got) == Some(t.date().to_string()) {
                    Eq::Day
                } else {
                    Eq::No
                }
            } else {
                yes_if(norm(&plain(sent)) == norm(&plain(got)))
            }
        }
        Value::Number(_) => yes_if(norm(&plain(sent)) == norm(&plain(got))),
        _ => Eq::No,
    }
}

fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Decimal text without trailing zeros (`"19.00"` and `19` agree).
fn norm(s: &str) -> String {
    let t = s.trim();
    let body = t.strip_prefix('-').unwrap_or(t);
    let numeric = body.chars().next().is_some_and(|c| c.is_ascii_digit())
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.matches('.').count() <= 1;
    if numeric && t.contains('.') {
        t.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        t.to_owned()
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

/// `yyyy-mm-dd` of a `dd.mm.yyyy` or `yyyy-mm-dd` string sent as a plain date.
fn sent_day(s: &str) -> Option<String> {
    let b = s.as_bytes();
    if b.len() == 10 && b[2] == b'.' && b[5] == b'.' {
        Some(format!(
            "{}-{}-{}",
            s.get(6..10)?,
            s.get(3..5)?,
            s.get(0..2)?
        ))
    } else if b.len() == 10 && b[4] == b'-' && b[7] == b'-' {
        Some(s.to_owned())
    } else {
        None
    }
}

fn unix_day(secs: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(secs)
        .ok()
        .map(|t| t.date().to_string())
}

/// The calendar day a read-back date states (in its own offset when it has one).
fn day_of(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => n.as_i64().and_then(unix_day),
        Value::String(s) => {
            let b = s.as_bytes();
            if b.len() >= 10 && b[4] == b'-' && b[7] == b'-' {
                s.get(..10).map(str::to_owned)
            } else if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                s.parse().ok().and_then(unix_day)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// The instant a read-back date states, as Unix seconds.
pub(crate) fn instant(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => OffsetDateTime::parse(s, &Rfc3339)
            .ok()
            .map(|t| t.unix_timestamp())
            .or_else(|| {
                (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                    .then(|| s.parse().ok())
                    .flatten()
            }),
        _ => None,
    }
}

/// A read-back date-time as `OffsetDateTime` (for the query values).
pub(crate) fn instant_of(v: &Value) -> OffsetDateTime {
    OffsetDateTime::parse(v.as_str().unwrap_or_default(), &Rfc3339)
        .unwrap_or_else(|e| panic!("read-back date-time {v}: {e}"))
}
