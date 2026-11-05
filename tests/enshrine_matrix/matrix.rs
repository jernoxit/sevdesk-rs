//! The order of the run: probe the finalizers, then every state of every kind.
#![allow(
    clippy::unwrap_used,
    reason = "test helper: a failure here is a test failure"
)]

use crate::build::{Doc, Env, Finalizer, Kind, State};
use crate::harness::*;
use crate::ops::Op;
use crate::record::{Outcome, St, out_dir};
use std::sync::Arc;
use time::OffsetDateTime;

const KINDS: [Kind; 2] = [Kind::Invoice, Kind::CreditNote];
const STATES: [State; 5] = [
    State::Draft,
    State::Finalized,
    State::Enshrined,
    State::Paid,
    State::TxEnshrined,
];

/// Operations of one state: those that cannot change the document first,
/// those that likely do last (a changed document is rebuilt for the rest).
fn ops_for(kind: Kind, state: State) -> Vec<Op> {
    let invoice = kind == Kind::Invoice;
    let mut ops = vec![Op::PdfPrevent, Op::Xml, Op::ContentPut, Op::ContentSave];
    let book = match state {
        State::Draft | State::Finalized | State::Enshrined => Op::Book,
        State::Paid | State::TxEnshrined => Op::Book2,
    };
    if state == State::TxEnshrined {
        ops.extend([Op::Enshrine, book, Op::DeleteTx, Op::ResetOpen, Op::Delete]);
        return ops;
    }
    ops.extend([Op::ChangeParam, Op::Enshrine]);
    if invoice {
        ops.push(Op::CreditNoteRef);
    }
    ops.push(book);
    if invoice {
        ops.push(Op::Cancel);
    }
    ops.extend([Op::ResetDraft, Op::PdfNoPrevent, Op::ResetOpen, Op::Delete]);
    ops
}

impl Env {
    /// Is "finalized, not enshrined" reachable, and how? Each way gets a fresh
    /// draft; the first that leaves status 200 without `enshrined` is kept.
    async fn probe_finalizers(&mut self) {
        for kind in KINDS {
            for how in Finalizer::ALL {
                let Ok(doc) = self.new_draft(kind, "probe", 1).await else {
                    self.setup_failed("probe draft");
                    continue;
                };
                self.setup_ok();
                let out = self.finalize(&doc, how).await;
                let after = self.read_state(kind, doc.id).await;
                self.rec
                    .row((kind.obj(), "draft", how.name()), &out, &after, doc.id);
                if after.status == "200" && !after.enshrined {
                    if how == Finalizer::SendBy {
                        // sendBy left it unenshrined: does /enshrine set it?
                        let out = self.enshrine_raw(&doc, "after_sendBy").await;
                        let after = self.read_state(kind, doc.id).await;
                        self.rec.row(
                            (kind.obj(), "finalized", "probe_enshrine_after_sendBy"),
                            &out,
                            &after,
                            doc.id,
                        );
                    }
                    self.finalizer[kind as usize] = Some(how);
                    break;
                }
            }
        }
    }

    /// A draft rendered without `preventSendBy`: run 1 left a credit note at
    /// status 200 without `enshrined` although the answer was 400/832.
    async fn half_finished(&mut self, kind: Kind) {
        let Ok(doc) = self.new_draft(kind, "half", 1).await else {
            return;
        };
        let out = self.finalize(&doc, Finalizer::PdfRender).await;
        let after = self.read_state(kind, doc.id).await;
        self.rec.row(
            (kind.obj(), "draft", "getPdf_no_preventSendBy_fresh"),
            &out,
            &after,
            doc.id,
        );
    }

    async fn run_state(&mut self, kind: Kind, state: State) {
        if state == State::Finalized && self.finalizer[kind as usize].is_none() {
            let out = Outcome {
                ok: false,
                status: 0,
                code: None,
                message: "not reachable: no probe left status 200 without enshrined".into(),
            };
            let none = St {
                status: "-".into(),
                enshrined: false,
                paid: "-".into(),
            };
            self.rec
                .row((kind.obj(), state.name(), "-"), &out, &none, 0);
            return;
        }
        let mut slot: Option<Doc> = None;
        for op in ops_for(kind, state) {
            if slot.is_none() {
                match self.build(kind, state).await {
                    Ok(doc) => {
                        self.setup_ok();
                        slot = Some(doc);
                    }
                    Err(e) => {
                        let out = Outcome {
                            ok: false,
                            status: 0,
                            code: None,
                            message: format!("SETUP FAILED: {e}"),
                        };
                        let none = St {
                            status: "-".into(),
                            enshrined: false,
                            paid: "-".into(),
                        };
                        self.rec
                            .row((kind.obj(), state.name(), op.name()), &out, &none, 0);
                        self.setup_failed(&e);
                        continue;
                    }
                }
            }
            let doc = slot.as_mut().unwrap();
            let out = self.run_op(doc, op).await;
            let after = self.read_state(kind, doc.id).await;
            self.rec
                .row((kind.obj(), state.name(), op.name()), &out, &after, doc.id);
            if after != doc.st || doc.dirty {
                slot = None;
            }
        }
    }
}

pub(crate) async fn run(label: &str) {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the enshrine matrix");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Arc::new(Ctx::new(&token, format!("SDRS-ENSH-{label}-{ts}")));
    let mut env = Env::new(label, ctx).await;
    env.probe_finalizers().await;
    for kind in KINDS {
        for state in STATES {
            env.run_state(kind, state).await;
        }
    }
    env.half_finished(Kind::CreditNote).await;
    eprintln!(
        "MATRIX_DONE run {label}: {} documents created (incl. origin, probes, cancellations); output in {}",
        env.created.get(),
        out_dir()
    );
}
