//! The probe's state and its recording: every step is one `NUMBERS\t…` line
//! on stderr and one row of the case's TSV.
use crate::build::{Env, Kind};
use crate::calls::Res;
use crate::docs::raw_one;
use crate::harness::*;
use reqwest::Method;
use serde::Deserialize;
use sevdesk::*;
use std::fs::File;
use std::future::Future;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;
use time::OffsetDateTime;

const HEADER: &str = "step\tkind\tid\thttp\tcode\tmessage\tnumber_in_answer\tnumber_reread\tseq_before\tseq_after\tstart_ms\tend_ms";

/// A number sequence as `GET /SevSequence/Factory/getByType` answers it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeqProbe {
    format: Option<String>,
    next_sequence: Option<String>,
}

/// An answer with the wall-clock span (ms since the case began) of its call.
pub(crate) struct Call {
    res: Res,
    start: u128,
    end: u128,
}

pub(crate) struct Probe {
    pub(crate) env: Env,
    /// A second client with its own pacing, for the concurrent cases.
    ctx2: Arc<Ctx>,
    tsv: File,
    clock: Instant,
    _alone: tokio::sync::MutexGuard<'static, ()>,
}

async fn timed(clock: Instant, f: impl Future<Output = Res>) -> Call {
    let start = clock.elapsed().as_millis();
    let res = f.await;
    Call {
        res,
        start,
        end: clock.elapsed().as_millis(),
    }
}

impl Probe {
    /// `None` without a token. `Env::new` aborts on any account but the test
    /// account, before the first write.
    pub(crate) async fn new(case: &str) -> Option<Self> {
        let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
            eprintln!("SEVDESK_API_KEY is not set: skipping the number probe");
            return None;
        };
        let alone = SERIAL.lock().await;
        let ts = OffsetDateTime::now_utc().unix_timestamp();
        let marker = format!("SDRS-NUM-{case}-{ts}");
        let ctx = Arc::new(Ctx::new(&token, marker.clone()));
        let ctx2 = Arc::new(Ctx::new(&token, marker));
        let env = Env::new(&format!("num-{case}"), ctx).await;
        let dir = format!("{}/number_probe", env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut tsv = File::create(format!("{dir}/{case}.tsv")).unwrap();
        writeln!(tsv, "{HEADER}").unwrap();
        Some(Self {
            env,
            ctx2,
            tsv,
            clock: Instant::now(),
            _alone: alone,
        })
    }

    pub(crate) fn ctx(&self, which: usize) -> Arc<Ctx> {
        if which == 0 {
            self.env.ctx.clone()
        } else {
            self.ctx2.clone()
        }
    }

    pub(crate) fn marker(&self) -> &str {
        &self.env.ctx.marker
    }

    /// A fresh enshrined invoice for a credit note to refer to.
    pub(crate) async fn origin(&mut self, tag: &str) -> InvoiceId {
        let doc = self.env.new_draft(Kind::Invoice, tag, 20).await.unwrap();
        self.env.enshrine_doc(&doc).await.unwrap();
        eprintln!("SETUP source invoice {} ({tag})", doc.id);
        InvoiceId::new(doc.id)
    }

    /// The next number of the kind's sequence, `format#next`.
    pub(crate) async fn seq(&self, kind: Kind) -> String {
        let (object, code) = match kind {
            Kind::Invoice => ("Invoice", "RE"),
            Kind::CreditNote => ("CreditNote", "CN"),
        };
        let path = format!("/SevSequence/Factory/getByType?objectType={object}&type={code}");
        let (status, text) = self.env.ctx.raw(Method::GET, &path, None, None).await;
        if status >= 300 {
            return format!("HTTP {status}");
        }
        match serde_json::from_value::<SeqProbe>(raw_one(&text)) {
            Ok(s) => format!(
                "{}#{}",
                s.format.unwrap_or_default(),
                s.next_sequence.unwrap_or_default()
            ),
            Err(e) => format!("unreadable: {e}"),
        }
    }

    async fn reread(&self, kind: Kind, id: i64) -> String {
        let field = match kind {
            Kind::Invoice => "invoiceNumber",
            Kind::CreditNote => "creditNoteNumber",
        };
        self.env.read_field(kind, id, field).await
    }

    fn write(&mut self, cols: &[String]) {
        let line = cols.join("\t");
        eprintln!("NUMBERS\t{line}");
        writeln!(self.tsv, "{line}").unwrap();
    }

    /// Writes the row of a finished call: re-reads the number by id and the
    /// sequence after it. Returns the answer with the id filled in.
    async fn record(
        &mut self,
        step: &str,
        kind: Kind,
        id: Option<i64>,
        call: Call,
        before: &str,
    ) -> Res {
        let id = call.res.id.or(id);
        let reread = match id {
            Some(i) => self.reread(kind, i).await,
            None => "-".into(),
        };
        let after = self.seq(kind).await;
        let out = &call.res.out;
        let cols = [
            step.to_owned(),
            kind.obj().to_owned(),
            id.map_or("-".into(), |i| i.to_string()),
            if out.ok {
                "ok".to_owned()
            } else {
                out.status.to_string()
            },
            out.code.map_or("-".into(), |c| c.to_string()),
            out.message.clone(),
            call.res.number.clone(),
            reread,
            before.to_owned(),
            after,
            call.start.to_string(),
            call.end.to_string(),
        ];
        self.write(&cols);
        Res { id, ..call.res }
    }

    /// One call, recorded with the sequence before and after.
    pub(crate) async fn step(
        &mut self,
        step: &str,
        kind: Kind,
        id: Option<i64>,
        f: impl Future<Output = Res>,
    ) -> Res {
        let before = self.seq(kind).await;
        let call = timed(self.clock, f).await;
        self.record(step, kind, id, call, &before).await
    }

    /// Two calls at the same time, one row each (suffix `_A` and `_B`).
    pub(crate) async fn pair(
        &mut self,
        step: &str,
        kind: Kind,
        ids: [Option<i64>; 2],
        fa: impl Future<Output = Res>,
        fb: impl Future<Output = Res>,
    ) -> [Res; 2] {
        let before = self.seq(kind).await;
        let (ca, cb) = tokio::join!(timed(self.clock, fa), timed(self.clock, fb));
        let a = self
            .record(&format!("{step}_A"), kind, ids[0], ca, &before)
            .await;
        let b = self
            .record(&format!("{step}_B"), kind, ids[1], cb, &before)
            .await;
        [a, b]
    }

    /// `f` on both documents at the same time (client 0 on A, client 1 on B);
    /// a document that was never created is skipped.
    pub(crate) async fn concurrent<F, Fut>(
        &mut self,
        step: &str,
        kind: Kind,
        ids: [Option<i64>; 2],
        f: F,
    ) where
        F: Fn(Arc<Ctx>, i64) -> Fut,
        Fut: Future<Output = Res>,
    {
        if let [Some(a), Some(b)] = ids {
            self.pair(step, kind, ids, f(self.ctx(0), a), f(self.ctx(1), b))
                .await;
            return;
        }
        for (which, id) in ids.into_iter().enumerate() {
            if let Some(id) = id {
                let fut = f(self.ctx(which), id);
                self.step(
                    &format!("{step}_{}", ["A", "B"][which]),
                    kind,
                    Some(id),
                    fut,
                )
                .await;
            }
        }
    }

    /// Looks for documents a failed or odd step may have left behind and
    /// records what is found: by `customerInternalNote`, by number, or (credit
    /// notes) by the source invoice they refer to.
    pub(crate) async fn half_written(
        &mut self,
        step: &str,
        kind: Kind,
        note: Option<&str>,
        number: Option<&str>,
        source: Option<InvoiceId>,
    ) {
        let ctx = self.ctx(0);
        let c = &ctx.c;
        let mut found = Vec::new();
        match kind {
            Kind::CreditNote => {
                let mut queries = vec![];
                if note.is_some() || number.is_some() {
                    queries.push(CreditNoteQuery {
                        customer_internal_note: note.map(str::to_owned),
                        credit_note_number: number.map(str::to_owned),
                        ..Default::default()
                    });
                }
                if source.is_some() {
                    queries.push(CreditNoteQuery {
                        page: Some(Page::new(100, 0).unwrap()),
                        ..Default::default()
                    });
                }
                for q in queries {
                    match c.list_credit_notes(&q).await {
                        Ok(list) => found.extend(
                            list.into_iter()
                                .filter(|n| {
                                    source.is_none_or(|s| {
                                        n.origin.as_ref().is_some_and(|o| o.id.get() == s.get())
                                    })
                                })
                                .map(|n| {
                                    format!(
                                        "CreditNote {} number {:?} status {:?}",
                                        n.id.get(),
                                        n.credit_note_number,
                                        n.status
                                    )
                                }),
                        ),
                        Err(e) => found.push(format!("list failed: {e}")),
                    }
                }
            }
            Kind::Invoice => {
                let q = InvoiceQuery {
                    customer_internal_note: note.map(str::to_owned),
                    invoice_number: number.map(str::to_owned),
                    ..Default::default()
                };
                match c.list_invoices(&q).await {
                    Ok(list) => found.extend(list.into_iter().map(|i| {
                        format!(
                            "Invoice {} number {:?} status {:?}",
                            i.id.get(),
                            i.invoice_number,
                            i.status
                        )
                    })),
                    Err(e) => found.push(format!("list failed: {e}")),
                }
            }
        }
        let message = format!("found {}: {}", found.len(), found.join("; "));
        let seq = self.seq(kind).await;
        let now = self.clock.elapsed().as_millis().to_string();
        let cols = [
            format!("half_written_{step}"),
            kind.obj().to_owned(),
            "-".into(),
            "ok".into(),
            "-".into(),
            message,
            "-".into(),
            "-".into(),
            seq.clone(),
            seq,
            now.clone(),
            now,
        ];
        self.write(&cols);
    }
}
