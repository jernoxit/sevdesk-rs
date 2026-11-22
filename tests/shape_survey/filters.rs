//! `filters_all`: a READ-ONLY measurement of every list filter the OpenAPI spec
//! names for the list endpoints the crate has (`GET /Invoice`, `/CreditNote`,
//! `/Voucher`, `/Contact`, `/CheckAccountTransaction`), plus the filters the
//! crate already sends as controls. sevDesk silently IGNORES unknown or
//! misspelled filters and answers with everything, so a filter counts only after
//! the filtered answer is compared with the unfiltered list. The filter table is
//! `filter_table.rs`; values come from the live data (`filter_cases.rs`); the
//! classification is `filter_judge.rs`.
//!
//! Classes: `exact`, `substring`, `prefix`, `range`, `ignored`, `wrong`,
//! `error`, `untestable`, and for filters without a per-object predicate
//! `narrows` / `widens` / `changed`, for orderings `order`.
//!
//! Output per endpoint: `target/tmp/shape_survey/filters_<Endpoint>.tsv` and one
//! stderr line per request `FILTER\t<endpoint>\t<filter>\t<class>\t<filtered>/<unfiltered>`
//! (`<filter> (<value form>)` when a filter is tried in several forms). No
//! value of the account is printed: only forms, lengths, counts and codes.
//!
//! `just test-live --test shape_survey filters_all`

use super::filter_cases::{Aux, cases};
use super::filter_judge::judge;
use super::filter_table::{ENDPOINTS, SPECS};
use super::harness::{Ctx, SERIAL};
use super::{MAX_PAGES, PAGE, get, objects_of, verify_test_account};
use serde_json::Value;
use std::time::Duration;

/// Percent-encodes everything but the unreserved characters (brackets too, as
/// the crate's own queries do).
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

/// Code and (truncated) message of a sevDesk error body.
fn error_note(text: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return "body is not JSON".into();
    };
    let e = v.get("error").filter(|e| !e.is_null()).unwrap_or(&v);
    let code = e.get("code").map(|c| c.to_string()).unwrap_or_default();
    let message = e.get("message").and_then(Value::as_str).unwrap_or_default();
    let message: String = message
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect();
    format!("code {code} message {message:?}")
}

/// GET of a list, all pages (at most `MAX_PAGES` of `PAGE`). An HTTP status of
/// 300 or more, an `error` in the body or a missing `objects` is an `Err`.
pub(crate) async fn fetch(
    ctx: &Ctx,
    path: &str,
    pairs: &[(String, String)],
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    for page in 0..MAX_PAGES {
        let mut q = pairs.to_vec();
        q.push(("limit".into(), PAGE.to_string()));
        q.push(("offset".into(), (page * PAGE).to_string()));
        let url = format!("{path}?{}", query(&q));
        let (mut status, mut text) = get(ctx, &url).await;
        for _ in 0..3 {
            if status != 429 {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
            (status, text) = get(ctx, &url).await;
        }
        if status >= 300 {
            return Err(format!("http {status}: {}", error_note(&text)));
        }
        if serde_json::from_str::<Value>(&text)
            .ok()
            .is_some_and(|v| v.get("error").is_some_and(|e| !e.is_null()))
        {
            return Err(format!(
                "http {status} with an error body: {}",
                error_note(&text)
            ));
        }
        let list = objects_of(&text, true).ok_or("answer has no objects array")?;
        let n = list.len();
        all.extend(list);
        if n < PAGE {
            break;
        }
    }
    Ok(all)
}

struct Row {
    filter: String,
    form: String,
    class: String,
    filtered: Option<usize>,
    unfiltered: usize,
    note: String,
    source: &'static str,
    spec_type: &'static str,
    field: &'static str,
}

/// A value sent must not come back in a note.
fn scrub(note: &str, pairs: &[(String, String)]) -> String {
    let mut out = note.to_owned();
    for (_, val) in pairs {
        if val.chars().count() >= 3 {
            out = out.replace(val.as_str(), "<value>");
        }
    }
    out
}

fn tsv(rows: &[Row]) -> String {
    let mut out = String::from(
        "filter\tvalue_form\tclass\tfiltered\tunfiltered\tnote\tsource\tspec_type\tfield\n",
    );
    for r in rows {
        let filtered = r.filtered.map_or("-".to_owned(), |n| n.to_string());
        out.push_str(&format!(
            "{}\t{}\t{}\t{filtered}\t{}\t{}\t{}\t{}\t{}\n",
            r.filter,
            r.form,
            r.class,
            r.unfiltered,
            r.note.replace(['\t', '\n'], " "),
            r.source,
            r.spec_type,
            r.field
        ));
    }
    out
}

async fn run_endpoint(ctx: &Ctx, endpoint: &str) {
    let path = format!("/{endpoint}");
    let mut rows: Vec<Row> = Vec::new();
    let unf = match fetch(ctx, &path, &[]).await {
        Ok(list) => list,
        Err(e) => {
            eprintln!("FILTER-NOTE\t{endpoint}\tunfiltered list failed: {e}");
            return;
        }
    };
    let mut aux = Aux::default();
    for spec in SPECS.iter().filter(|s| s.endpoint == endpoint) {
        let source = if spec.from_crate { "crate" } else { "spec" };
        let mut push = |form: String, class: &str, filtered: Option<usize>, note: String| {
            let several = !form.is_empty();
            let label = if several {
                format!("{} ({form})", spec.filter)
            } else {
                spec.filter.to_owned()
            };
            let shown = filtered.map_or("-".to_owned(), |n| n.to_string());
            eprintln!(
                "FILTER\t{endpoint}\t{label}\t{class}\t{shown}/{}",
                unf.len()
            );
            rows.push(Row {
                filter: spec.filter.to_owned(),
                form,
                class: class.to_owned(),
                filtered,
                unfiltered: unf.len(),
                note,
                source,
                spec_type: spec.spec_type,
                field: spec.field,
            });
        };
        let cases = match cases(ctx, &mut aux, spec, &unf).await {
            Ok(c) => c,
            Err(reason) => {
                push(String::new(), "untestable", None, reason);
                continue;
            }
        };
        for case in &cases {
            match fetch(ctx, &path, &case.pairs).await {
                Err(e) => push(case.form.clone(), "error", None, scrub(&e, &case.pairs)),
                Ok(filt) => {
                    let verdict = judge(spec, case, &unf, &filt);
                    push(
                        case.form.clone(),
                        verdict.class,
                        Some(filt.len()),
                        verdict.note,
                    );
                }
            }
        }
    }
    let dir = format!("{}/shape_survey", env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/filters_{endpoint}.tsv"), tsv(&rows)).unwrap();
}

#[tokio::test]
#[ignore = "reads the whole sevDesk test account"]
async fn filters_all() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping filters_all");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ctx = Ctx::new(&token, "shape-survey".into());
    verify_test_account(&ctx).await;
    for endpoint in ENDPOINTS {
        run_endpoint(&ctx, endpoint).await;
    }
}
