//! The per-object loop: one field at a time, candidates in order, read back
//! after every write, one TSV row per attempt.

use crate::classify::classify;
use crate::harness::Ctx;
use crate::table::{Phase, Row, Scope};
use crate::target::{Snap, Target};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::io::Write;
use time::macros::datetime;

pub(crate) struct Env<'a> {
    pub(crate) ctx: &'a Ctx,
    /// Objects found read-only, by sevDesk object name.
    pub(crate) refs: BTreeMap<&'static str, i64>,
}

fn out_dir() -> String {
    format!("{}/field_probe", env!("CARGO_TARGET_TMPDIR"))
}

struct Tsv(String);

impl Tsv {
    fn new(key: &str) -> Self {
        std::fs::create_dir_all(format!("{}/captures", out_dir())).unwrap();
        let path = format!("{}/{key}.tsv", out_dir());
        std::fs::write(
            &path,
            "field\tvalue sent\tencoding\tclass\tread-back form\tnote\n",
        )
        .unwrap();
        Self(path)
    }

    fn row(&self, cols: [&str; 6]) {
        let line: Vec<String> = cols
            .iter()
            .map(|c| c.replace(['\t', '\n', '\r'], " "))
            .collect();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.0)
            .unwrap();
        writeln!(f, "{}", line.join("\t")).unwrap();
    }
}

fn subst(text: &str, env: &Env<'_>, t: &Target) -> Result<String, String> {
    let unix = datetime!(2026-11-20 12:00 UTC).unix_timestamp();
    let mut s = text
        .replace("@dmy@", "20.11.2026")
        .replace("@unix@", &unix.to_string())
        .replace("@rfc@", "2026-11-20T12:00:00+01:00")
        .replace("@marker@", &env.ctx.marker)
        .replace(
            "@self@",
            &format!(r#"{{"id":{},"objectName":"{}"}}"#, t.id, t.object),
        );
    while let Some(i) = s.find("@ref:") {
        let rest = &s[i + 5..];
        let end = rest.find('@').ok_or("unterminated @ref: token")?;
        let name = &rest[..end];
        let id = env
            .refs
            .get(name)
            .ok_or_else(|| format!("no {name} object found read-only in the account"))?;
        let rep = format!(r#"{{"id":{id},"objectName":"{name}"}}"#);
        s.replace_range(i..i + 5 + end + 1, &rep);
    }
    Ok(s)
}

fn head_of(label: &str, v: &Value) -> String {
    v.get(label)
        .map_or_else(|| "<absent>".to_owned(), Value::to_string)
}

/// `status`, number, `enshrined`, `sendDate` of a head, for the state fields.
fn state_summary(head: &Value) -> String {
    let get = |k: &str| head_of(k, head);
    format!(
        "status={} invoiceNumber={} creditNoteNumber={} enshrined={} sendDate={} payDate={}",
        get("status"),
        get("invoiceNumber"),
        get("creditNoteNumber"),
        get("enshrined"),
        get("sendDate"),
        get("payDate")
    )
}

fn merge_extra(patch: &mut Map<String, Value>, extra: &str) {
    if extra.is_empty() {
        return;
    }
    let Value::Object(m) = serde_json::from_str(extra).expect("candidate extra is JSON") else {
        panic!("candidate extra is no object");
    };
    patch.extend(m);
}

/// Measures every row of the object: position fields, head fields, then the
/// state fields.
pub(crate) async fn run_object(env: &Env<'_>, t: &Target, rows: &[Row]) {
    let tsv = Tsv::new(t.key);
    let mine = || rows.iter().filter(|r| r.object == t.key);
    let ordered: Vec<&Row> = mine()
        .filter(|r| r.phase == Phase::Normal && r.scope == Scope::Pos)
        .chain(mine().filter(|r| r.phase == Phase::Normal && r.scope == Scope::Head))
        .chain(mine().filter(|r| r.phase == Phase::State))
        .collect();
    eprintln!("OBJECT\t{}\t{} fields", t.key, ordered.len());
    let empty = Value::Object(Map::new());
    for row in ordered {
        let label = match row.scope {
            Scope::Head => row.field.to_owned(),
            Scope::Pos => format!("pos.{}", row.field),
        };
        let mut snap: Snap = t.snapshot(env.ctx).await;
        let mut classes: Vec<&'static str> = Vec::new();
        let mut last_note = String::new();
        for (n, cand) in row.candidates.iter().enumerate() {
            let text = match subst(cand.value, env, t) {
                Ok(s) => s,
                Err(why) => {
                    tsv.row([&label, "-", cand.encoding, "untestable", "-", &why]);
                    classes.push("untestable");
                    last_note = why;
                    break;
                }
            };
            let sent: Value = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("candidate of {label} is no JSON: {e}"));
            let mut patch = Map::new();
            patch.insert(row.field.to_owned(), sent.clone());
            merge_extra(&mut patch, cand.extra);
            let (status, body) = t.send(env.ctx, row.scope, patch).await;
            let failed = status >= 300 || body.contains("\"error\"");
            let scrubbed = crate::scrub::scrub_capture(&body);
            if failed {
                let file = format!(
                    "{}/captures/{}_{}_{n}.json",
                    out_dir(),
                    t.key,
                    label.replace('.', "_")
                );
                std::fs::write(file, &scrubbed).unwrap();
            }
            // Re-read always: an error answer does not mean "not written".
            let after = t.snapshot(env.ctx).await;
            let (before_c, after_c) = match row.scope {
                Scope::Head => (&snap.head, &after.head),
                Scope::Pos => (
                    snap.pos.as_ref().unwrap_or(&empty),
                    after.pos.as_ref().unwrap_or(&empty),
                ),
            };
            let (mut class, mut note) =
                classify(row.kind, row.field, &sent, before_c, after_c, failed);
            if failed {
                let head: String = scrubbed.chars().take(300).collect();
                note = format!("HTTP {status} {head}; {note}");
            }
            if after.pos_count != snap.pos_count {
                note.push_str(&format!(
                    "; position count {} -> {}",
                    snap.pos_count, after.pos_count
                ));
                if class != "refused" {
                    class = "changed-other";
                }
            }
            let read_back: String = head_of(row.field, after_c).chars().take(200).collect();
            tsv.row([
                &label,
                &sent.to_string(),
                cand.encoding,
                class,
                &read_back,
                &note,
            ]);
            classes.push(class);
            last_note = format!("{} [{}]; read back {read_back}", note, cand.encoding);
            let stop_state = row.phase == Phase::State
                && row.field == "status"
                && snap.head.get("status") != after.head.get("status");
            if row.phase == Phase::State {
                let summary = state_summary(&after.head);
                eprintln!("STATE\t{}\t{label}\t{summary}", t.key);
                tsv.row([&label, "-", cand.encoding, "state-after", "-", &summary]);
            }
            if stop_state {
                let key = if t.object == "CreditNote" {
                    "creditNoteNumber"
                } else {
                    "invoiceNumber"
                };
                eprintln!(
                    "NUMBER {} {} {}\t{}",
                    t.object,
                    t.id,
                    head_of(key, &after.head),
                    state_summary(&after.head)
                );
            }
            snap = after;
            if class == "stored" || stop_state {
                break;
            }
        }
        let mut distinct = classes.clone();
        distinct.dedup();
        let class = if classes.contains(&"stored") {
            "stored".to_owned()
        } else {
            distinct.join("/")
        };
        eprintln!(
            "FIELD\t{}\t{label}\t{class}\t{}",
            t.key,
            last_note.replace('\n', " ")
        );
    }
}
