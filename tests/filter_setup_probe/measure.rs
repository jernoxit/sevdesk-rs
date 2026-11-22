//! Read-only measurement helpers of the filter setup probe: list an endpoint
//! with a filter, compare the filtered answer with the sets that stated
//! predicates select from the unfiltered list, and record one row per request.
//! Output carries counts, class names and codes, never a value of the account.

use super::harness::Ctx;
use reqwest::Method;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

pub(crate) type Pairs = Vec<(String, String)>;

const PAGE: usize = 1000;
const MAX_PAGES: usize = 10;

pub(crate) fn pairs(items: &[(&str, &str)]) -> Pairs {
    items
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// Percent-encodes everything but the unreserved characters (brackets too).
fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn query(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// An error in the body (also inside HTTP 200) or a status of 300 or more.
pub(crate) fn failed(status: u16, text: &str) -> bool {
    status >= 300
        || serde_json::from_str::<Value>(text)
            .ok()
            .is_some_and(|v| v.get("error").is_some_and(|e| !e.is_null()))
}

/// Status, code and (truncated) message of an answer, for a note.
pub(crate) fn error_note(status: u16, text: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return format!("http {status}, body is not JSON");
    };
    let e = v.get("error").filter(|e| !e.is_null()).unwrap_or(&v);
    let code = e.get("code").map(|c| c.to_string()).unwrap_or_default();
    let message: String = e
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect();
    format!("http {status} code {code} message {message:?}")
}

/// The first object of an answer (`objects` as object or as list).
pub(crate) fn first_object(text: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(text).ok()?;
    match v.get("objects")? {
        Value::Array(list) => list.first().cloned(),
        o @ Value::Object(_) => Some(o.clone()),
        _ => None,
    }
}

/// GET of a list, all pages. Read-only; a 429 is waited out (reads are safe to repeat).
pub(crate) async fn list(ctx: &Ctx, path: &str, filter: &Pairs) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    for page in 0..MAX_PAGES {
        let mut q = filter.clone();
        q.push(("limit".into(), PAGE.to_string()));
        q.push(("offset".into(), (page * PAGE).to_string()));
        let url = format!("{path}?{}", query(&q));
        let (mut status, mut text) = ctx.raw(Method::GET, &url, None, None).await;
        for _ in 0..3 {
            if status != 429 {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
            (status, text) = ctx.raw(Method::GET, &url, None, None).await;
        }
        if failed(status, &text) {
            return Err(error_note(status, &text));
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let Some(Value::Array(objects)) = v.get("objects").cloned() else {
            return Err("answer has no objects array".into());
        };
        let n = objects.len();
        all.extend(objects);
        if n < PAGE {
            break;
        }
    }
    Ok(all)
}

pub(crate) fn sid(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn text(o: &Value, field: &str) -> Option<String> {
    sid(&o[field])
}

/// The `id` of a reference field `{id, objectName}`.
pub(crate) fn ref_id(o: &Value, field: &str) -> Option<String> {
    sid(&o[field]["id"])
}

pub(crate) fn ids(list: &[Value]) -> BTreeSet<String> {
    list.iter().filter_map(|o| sid(&o["id"])).collect()
}

/// A named predicate over the unfiltered list: the ids it selects.
pub(crate) struct Hyp {
    name: String,
    set: BTreeSet<String>,
}

pub(crate) fn hyp(name: &str, unf: &[Value], f: impl Fn(&Value) -> bool) -> Hyp {
    Hyp {
        name: name.to_owned(),
        set: unf
            .iter()
            .filter(|o| f(o))
            .filter_map(|o| sid(&o["id"]))
            .collect(),
    }
}

/// Class and note of a filtered answer against the hypotheses.
pub(crate) fn classify(unf: &[Value], filt: &[Value], hyps: &[Hyp]) -> (String, String) {
    let u = ids(unf);
    let f = ids(filt);
    let outside = f.difference(&u).count();
    let fits: Vec<&str> = hyps
        .iter()
        .filter(|h| h.set == f && outside == 0)
        .map(|h| h.name.as_str())
        .collect();
    let selected = hyps.iter().map(|h| h.set.len()).max().unwrap_or(0);
    let (class, note) = if hyps.is_empty() {
        if f == u {
            ("ignored", "answer equals the unfiltered list".to_owned())
        } else {
            (
                "changed",
                format!(
                    "answer differs from the unfiltered list: {} objects outside, {} missing",
                    outside,
                    u.difference(&f).count()
                ),
            )
        }
    } else if hyps.iter().all(|h| h.set.is_empty()) {
        ("untestable", "no object satisfies the predicate".to_owned())
    } else if f == u && hyps.iter().all(|h| h.set != u) {
        ("ignored", "answer equals the unfiltered list".to_owned())
    } else if !fits.is_empty() && f != u {
        ("exact", format!("fits: {}", fits.join(" | ")))
    } else if !fits.is_empty() {
        (
            "untestable",
            format!("the predicate selects everything: {}", fits.join(" | ")),
        )
    } else if f.is_empty() {
        ("empty", format!("predicate selects up to {selected}"))
    } else if outside > 0 {
        ("wrong", format!("{outside} objects outside the list"))
    } else {
        let best = hyps
            .iter()
            .map(|h| (h.set.symmetric_difference(&f).count(), h.name.as_str()))
            .min()
            .unwrap_or((0, ""));
        (
            "narrows",
            format!("no fit; closest diff {} ({})", best.0, best.1),
        )
    };
    (class.to_owned(), note)
}

pub(crate) struct Row {
    pub(crate) endpoint: String,
    pub(crate) filter: String,
    pub(crate) setup: String,
    pub(crate) form: String,
    pub(crate) class: String,
    pub(crate) filtered: Option<usize>,
    pub(crate) unfiltered: usize,
    pub(crate) note: String,
}

#[derive(Default)]
pub(crate) struct Results {
    rows: Vec<Row>,
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn counts(r: &Row) -> String {
    r.filtered.map_or_else(
        || format!("-/{}", r.unfiltered),
        |n| format!("{n}/{}", r.unfiltered),
    )
}

/// What a measured request is about.
pub(crate) struct Probe<'a> {
    pub(crate) endpoint: &'a str,
    pub(crate) filter: &'a str,
    pub(crate) setup: &'a str,
    pub(crate) form: &'a str,
    pub(crate) query: Pairs,
}

impl Results {
    pub(crate) fn push(&mut self, row: Row) {
        eprintln!(
            "FILTER2\t{}\t{}\t{}\t{}\t{}\t{}",
            row.endpoint,
            row.filter,
            row.form,
            row.class,
            counts(&row),
            clean(&row.note)
        );
        self.rows.push(row);
    }

    /// A filter that was not measured, with the reason.
    pub(crate) fn skipped(&mut self, endpoint: &str, filter: &str, class: &str, note: &str) {
        self.push(Row {
            endpoint: endpoint.into(),
            filter: filter.into(),
            setup: "-".into(),
            form: "-".into(),
            class: class.into(),
            filtered: None,
            unfiltered: 0,
            note: note.into(),
        });
    }

    /// One request: the filter in one value form, judged against `hyps`.
    pub(crate) async fn measure(
        &mut self,
        ctx: &Ctx,
        probe: Probe<'_>,
        unf: &[Value],
        hyps: &[Hyp],
    ) -> Option<Vec<Value>> {
        let result = list(ctx, &format!("/{}", probe.endpoint), &probe.query).await;
        let (class, note, filtered, got) = match result {
            Ok(filt) => {
                let (class, note) = classify(unf, &filt, hyps);
                (class, note, Some(filt.len()), Some(filt))
            }
            Err(e) => ("error".to_owned(), e, None, None),
        };
        self.push(Row {
            endpoint: probe.endpoint.into(),
            filter: probe.filter.into(),
            setup: probe.setup.into(),
            form: probe.form.into(),
            class,
            filtered,
            unfiltered: unf.len(),
            note,
        });
        got
    }

    /// The class of the last row that measured `endpoint` / `filter`.
    pub(crate) fn classes(&self, endpoint: &str, filter: &str) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| r.endpoint == endpoint && r.filter == filter)
            .map(|r| r.class.as_str())
            .collect()
    }

    pub(crate) fn write(&self) {
        let dir = format!("{}/filter_setup_probe", env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut out =
            String::from("endpoint\tfilter\tsetup\tvalue form\tclass\tfiltered/unfiltered\tnote\n");
        for r in &self.rows {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                r.endpoint,
                r.filter,
                clean(&r.setup),
                clean(&r.form),
                r.class,
                counts(r),
                clean(&r.note)
            ));
        }
        std::fs::write(format!("{dir}/results.tsv"), out).unwrap();
    }
}
