//! The probe's state and its recording: every step is one `LENGTHS\t…` line
//! on stderr and one row of the case's TSV.
use crate::build::{Env, Kind};
use crate::content::{first_diff, tail};
use crate::docs::raw_objects;
use crate::harness::*;
use crate::record::Outcome;
use crate::scrub::scrub_capture;
use reqwest::Method;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use time::OffsetDateTime;

const HEADER: &str = "step\tkind\tfield\tlevel\tid\thttp\tcode\tmessage\thalf_written\tsent_chars\tsent_bytes\tgot_chars\tgot_bytes\tequal\tfirst_diff\ttail";

pub(crate) struct Probe {
    pub(crate) env: Env,
    tsv: File,
    _alone: tokio::sync::MutexGuard<'static, ()>,
}

/// One row; the columns of [`HEADER`] that a step does not have stay `-`.
pub(crate) struct Row {
    pub(crate) step: String,
    pub(crate) kind: Kind,
    pub(crate) field: &'static str,
    pub(crate) level: String,
    pub(crate) id: Option<i64>,
    pub(crate) out: Outcome,
    pub(crate) half_written: String,
    /// What was sent and what a re-read returned (`None`: not re-read).
    pub(crate) sent: Option<String>,
    pub(crate) got: Option<String>,
    /// Overrides the `equal` column (the XML check).
    pub(crate) equal: Option<String>,
    pub(crate) note: Option<String>,
}

impl Row {
    pub(crate) fn new(step: &str, kind: Kind, field: &'static str, level: &str) -> Self {
        Self {
            step: step.to_owned(),
            kind,
            field,
            level: level.to_owned(),
            id: None,
            out: Outcome::ok(""),
            half_written: "-".into(),
            sent: None,
            got: None,
            equal: None,
            note: None,
        }
    }
}

/// The re-read of a position: the number of positions and the field's value.
pub(crate) struct Reread {
    pub(crate) count: usize,
    pub(crate) value: Option<String>,
}

impl Probe {
    /// `None` without a token. `Env::new` aborts on any account but the test
    /// account, before the first write.
    pub(crate) async fn new(case: &str) -> Option<Self> {
        let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
            eprintln!("SEVDESK_API_KEY is not set: skipping the length probe");
            return None;
        };
        let alone = SERIAL.lock().await;
        let ts = OffsetDateTime::now_utc().unix_timestamp();
        let ctx = Arc::new(Ctx::new(&token, format!("SDRS-LEN-{ts}")));
        let env = Env::new(&format!("len-{case}"), ctx).await;
        let dir = format!("{}/length_probe", env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut tsv = File::create(format!("{dir}/{case}.tsv")).unwrap();
        writeln!(tsv, "{HEADER}").unwrap();
        Some(Self {
            env,
            tsv,
            _alone: alone,
        })
    }

    pub(crate) fn marker(&self) -> &str {
        &self.env.ctx.marker
    }

    pub(crate) fn write(&mut self, r: Row) {
        let dash = || "-".to_owned();
        let (sent_c, sent_b) = r.sent.as_ref().map_or((dash(), dash()), |s| {
            (s.chars().count().to_string(), s.len().to_string())
        });
        let (got_c, got_b) = r.got.as_ref().map_or((dash(), dash()), |s| {
            (s.chars().count().to_string(), s.len().to_string())
        });
        let (equal, diff, tail_col) = match (&r.sent, &r.got) {
            (Some(s), Some(g)) => {
                let d = first_diff(s, g);
                (
                    (d.is_none()).to_string(),
                    d.map_or(dash(), |i| i.to_string()),
                    if d.is_some() { tail(g, 30) } else { dash() },
                )
            }
            _ => (dash(), dash(), dash()),
        };
        let cols = [
            r.step,
            r.kind.obj().to_owned(),
            r.field.to_owned(),
            r.level,
            r.id.map_or(dash(), |i| i.to_string()),
            if r.out.ok {
                "ok".to_owned()
            } else {
                r.out.status.to_string()
            },
            r.out.code.map_or(dash(), |c| c.to_string()),
            r.out.message.replace(['\t', '\n', '\r'], " "),
            r.half_written,
            sent_c,
            sent_b,
            got_c,
            got_b,
            r.equal.unwrap_or(equal),
            diff,
            r.note.unwrap_or(tail_col),
        ];
        let line = cols.join("\t");
        eprintln!("LENGTHS\t{line}");
        writeln!(self.tsv, "{line}").unwrap();
    }

    /// The positions of a document by their OWN GET, `field` of the first.
    /// Panics (with the scrubbed body) when there is no position at all.
    pub(crate) async fn reread(&self, kind: Kind, id: i64, field: &str) -> Reread {
        let path = match kind {
            Kind::Invoice => format!("/InvoicePos?invoice[id]={id}&invoice[objectName]=Invoice"),
            Kind::CreditNote => {
                format!("/CreditNotePos?creditNote[id]={id}&creditNote[objectName]=CreditNote")
            }
        };
        let (status, text) = self.env.ctx.raw(Method::GET, &path, None, None).await;
        let shown = || scrub_capture(&text.chars().take(2000).collect::<String>());
        assert!(status < 300, "GET {path} answered {status}: {}", shown());
        let list = raw_objects(&text);
        let positions = list.as_array().cloned().unwrap_or_default();
        eprintln!(
            "LENGTHS\tRAW_POSITIONS\t{}\t{id}\t{}",
            kind.obj(),
            positions.len()
        );
        assert!(
            !positions.is_empty(),
            "GET {path} returned no position: {}",
            shown()
        );
        Reread {
            count: positions.len(),
            value: Some(positions[0][field].as_str().unwrap_or_default().to_owned()),
        }
    }

    /// Looks for a document a failed save may have left behind, by
    /// `customerInternalNote`.
    pub(crate) async fn half_written(&self, kind: Kind, note: &str) -> String {
        let c = &self.env.ctx.c;
        let found: Result<Vec<String>, String> = match kind {
            Kind::Invoice => c
                .list_invoices(&sevdesk::InvoiceQuery {
                    customer_internal_note: Some(note.to_owned()),
                    ..Default::default()
                })
                .await
                .map(|l| {
                    l.iter()
                        .map(|i| format!("Invoice {}", i.id.get()))
                        .collect()
                })
                .map_err(|e| e.to_string()),
            Kind::CreditNote => c
                .list_credit_notes(&sevdesk::CreditNoteQuery {
                    customer_internal_note: Some(note.to_owned()),
                    ..Default::default()
                })
                .await
                .map(|l| {
                    l.iter()
                        .map(|n| format!("CreditNote {}", n.id.get()))
                        .collect()
                })
                .map_err(|e| e.to_string()),
        };
        match found {
            Ok(v) => format!("found {}: {}", v.len(), v.join("; ")),
            Err(e) => format!("list failed: {e}"),
        }
    }
}
