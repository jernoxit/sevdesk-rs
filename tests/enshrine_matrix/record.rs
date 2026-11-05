//! What one operation answered, and where the rows go.
#![allow(
    clippy::unwrap_used,
    reason = "test helper: a failure here is a test failure"
)]

use crate::scrub::scrub_capture;
use sevdesk::SevdeskError;
use std::collections::HashSet;
use std::io::Write;

pub(crate) fn out_dir() -> String {
    format!("{}/enshrine_matrix", env!("CARGO_TARGET_TMPDIR"))
}

/// The answer of an operation: HTTP status, sevDesk's `error.code`, a short
/// message. `ok` is false for any error answer, also one inside HTTP 200.
#[derive(Debug, Clone)]
pub(crate) struct Outcome {
    pub(crate) ok: bool,
    pub(crate) status: u16,
    pub(crate) code: Option<i64>,
    pub(crate) message: String,
}

fn short(s: &str) -> String {
    s.replace(['\n', '\r', '\t'], " ")
        .chars()
        .take(160)
        .collect()
}

impl Outcome {
    pub(crate) fn ok(message: impl AsRef<str>) -> Self {
        Self {
            ok: true,
            status: 200,
            code: None,
            message: short(message.as_ref()),
        }
    }

    /// A typed call's error. The typed client does not tell an error inside
    /// HTTP 200 from a real status; `Api.status` is then the `responseCode`.
    pub(crate) fn from_error(e: &SevdeskError) -> Self {
        let (status, code, message) = match e {
            SevdeskError::Api {
                status,
                code,
                message,
            } => (*status, *code, message.clone()),
            SevdeskError::NotFound { message } => (404, Some(151), message.clone()),
            other => (0, None, other.to_string()),
        };
        Self {
            ok: false,
            status,
            code,
            message: short(&message),
        }
    }

    /// A raw call's answer; an `error` object counts also inside HTTP 200.
    pub(crate) fn from_raw(status: u16, text: &str) -> Self {
        let v: Option<serde_json::Value> = serde_json::from_str(text).ok();
        let err = v.as_ref().and_then(|v| v.get("error"));
        if status < 300 && err.is_none() {
            return Self {
                ok: true,
                status,
                code: None,
                message: short(text),
            };
        }
        let num = |k: &str| {
            err.and_then(|e| e.get(k)).and_then(|c| {
                c.as_i64()
                    .or_else(|| c.as_str().and_then(|s| s.parse().ok()))
            })
        };
        let message = err
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or(text);
        let inside_200 = status < 300;
        Self {
            ok: false,
            status: if inside_200 {
                num("responseCode")
                    .and_then(|c| u16::try_from(c).ok())
                    .unwrap_or(status)
            } else {
                status
            },
            code: num("code"),
            message: short(&format!(
                "{}{message}",
                if inside_200 {
                    "[error inside HTTP 200] "
                } else {
                    ""
                }
            )),
        }
    }
}

/// The state axes of a document: status, enshrined, paid amount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct St {
    pub(crate) status: String,
    pub(crate) enshrined: bool,
    pub(crate) paid: String,
}

impl std::fmt::Display for St {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}",
            self.status,
            if self.enshrined { "enshrined" } else { "-" },
            self.paid
        )
    }
}

/// Prints each row as `MATRIX\t…`, appends it to the TSV, and keeps the first
/// refusal of every distinct (status, code) as a file.
pub(crate) struct Recorder {
    run: String,
    tsv: std::fs::File,
    seen: HashSet<(u16, Option<i64>)>,
}

impl Recorder {
    pub(crate) fn new(run: &str) -> Self {
        let dir = out_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let mut tsv = std::fs::File::create(format!("{dir}/ensh-{run}.tsv")).unwrap();
        writeln!(
            tsv,
            "run\tkind\tstate\toperation\thttp\tcode\tmessage\tafter_status\tafter_enshrined\tafter_paid\tdoc_id"
        )
        .unwrap();
        Self {
            run: run.into(),
            tsv,
            seen: HashSet::new(),
        }
    }

    pub(crate) fn row(
        &mut self,
        (kind, state, op): (&str, &str, &str),
        out: &Outcome,
        after: &St,
        doc_id: i64,
    ) {
        let code = out.code.map_or("-".to_owned(), |c| c.to_string());
        let line = format!(
            "{}\t{kind}\t{state}\t{op}\t{}\t{code}\t{}\t{after}\t{doc_id}",
            self.run, out.status, out.message
        );
        eprintln!("MATRIX\t{line}");
        writeln!(self.tsv, "{line}").unwrap();
        if !out.ok && self.seen.insert((out.status, out.code)) {
            let body = format!(
                "kind={kind} state={state} op={op}\nHTTP {} code {code}\n{}\n",
                out.status, out.message
            );
            std::fs::write(
                format!(
                    "{}/{}-refusal-{}-{code}.txt",
                    out_dir(),
                    self.run,
                    out.status
                ),
                scrub_capture(&body),
            )
            .unwrap();
        }
    }
}
