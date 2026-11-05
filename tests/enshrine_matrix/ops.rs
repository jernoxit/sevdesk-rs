//! The operations of the matrix, each answering with an [`Outcome`].
#![allow(
    clippy::unwrap_used,
    reason = "test helper: a failure here is a test failure"
)]

use crate::build::{Doc, Env, Finalizer, Kind};
use crate::docs::json;
use crate::harness::*;
use crate::record::Outcome;
use reqwest::Method;
use serde::Serialize;
use sevdesk::*;
use time::OffsetDateTime;
use time::macros::datetime;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Op {
    PdfPrevent,
    Xml,
    ContentPut,
    ContentSave,
    ChangeParam,
    Enshrine,
    PdfNoPrevent,
    CreditNoteRef,
    ResetOpen,
    ResetDraft,
    Cancel,
    Book,
    Book2,
    Delete,
    DeleteTx,
}

impl Op {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::PdfPrevent => "getPdf_preventSendBy",
            Self::Xml => "getXml",
            Self::ContentPut => "PUT_content",
            Self::ContentSave => "save_with_id",
            Self::ChangeParam => "changeParameter_en_US",
            Self::Enshrine => "enshrine",
            Self::PdfNoPrevent => "getPdf_no_preventSendBy",
            Self::CreditNoteRef => "saveCreditNote_ref",
            Self::ResetOpen => "resetToOpen",
            Self::ResetDraft => "resetToDraft",
            Self::Cancel => "cancelInvoice",
            Self::Book => "bookAmount",
            Self::Book2 => "bookAmount_again",
            Self::Delete => "DELETE",
            Self::DeleteTx => "DELETE_transaction",
        }
    }
}

#[derive(Serialize)]
struct HeaderBody {
    header: String,
}

#[derive(Serialize)]
struct StatusBody {
    status: i64,
}

#[derive(Serialize)]
struct ChangeStatusBody {
    value: i64,
}

fn pdf_summary(pdf: &DocumentPdf) -> String {
    format!("{} bytes, filename {:?}", pdf.bytes.len(), pdf.filename)
}

impl Env {
    async fn raw_put<B: Serialize>(&self, path: &str, body: Option<&B>) -> Outcome {
        let (s, t) = self.ctx.raw(Method::PUT, path, body.map(json), None).await;
        Outcome::from_raw(s, &t)
    }

    /// `PUT …/enshrine`, raw so the body can be recorded.
    pub(crate) async fn enshrine_raw(&self, doc: &Doc, tag: &str) -> Outcome {
        let name = format!(
            "{}_enshrine_{}_{}_{tag}",
            self.run,
            doc.kind.obj(),
            doc.st.status
        );
        let (s, t) = self
            .ctx
            .raw(
                Method::PUT,
                &format!("/{}/{}/enshrine", doc.kind.obj(), doc.id),
                None,
                Some(&name),
            )
            .await;
        Outcome::from_raw(s, &t)
    }

    /// Tries to take a draft out of status 100 by `how`.
    pub(crate) async fn finalize(&self, doc: &Doc, how: Finalizer) -> Outcome {
        let path = format!("/{}/{}", doc.kind.obj(), doc.id);
        let send = |draft| SendBy {
            send_draft: draft,
            ..SendBy::FINALIZE
        };
        match how {
            Finalizer::SendBy | Finalizer::SendDraft => {
                let send = send(how == Finalizer::SendDraft);
                Self::outcome_of(match doc.kind {
                    Kind::Invoice => self
                        .ctx
                        .c
                        .send_invoice_by(InvoiceId::new(doc.id), &send)
                        .await
                        .map(|i| format!("status {:?}", i.status)),
                    Kind::CreditNote => self
                        .ctx
                        .c
                        .send_credit_note_by(CreditNoteId::new(doc.id), &send)
                        .await
                        .map(|c| format!("status {:?}", c.status)),
                })
            }
            Finalizer::PdfRender => {
                let (s, t) = self
                    .ctx
                    .raw(Method::GET, &format!("{path}/getPdf"), None, None)
                    .await;
                Outcome::from_raw(s, &t)
            }
            Finalizer::PutStatus => self.raw_put(&path, Some(&StatusBody { status: 200 })).await,
            Finalizer::ChangeStatus => {
                self.raw_put(
                    &format!("{path}/changeStatus"),
                    Some(&ChangeStatusBody { value: 200 }),
                )
                .await
            }
        }
    }

    /// Books the whole document with a fresh (or kept) transaction on the
    /// online test account. A credit note books the negative amount.
    pub(crate) async fn book(&self, doc: &mut Doc) -> Outcome {
        let signed = match doc.kind {
            Kind::Invoice => doc.gross,
            Kind::CreditNote => -doc.gross,
        };
        let at = datetime!(2026-11-05 12:00 +1);
        let tx = match doc.spare.take() {
            Some(tx) => tx,
            None => match self
                .ctx
                .c
                .create_transaction(&self.ctx.new_transaction(ZEIT, signed, at))
                .await
            {
                Ok(t) => t.id,
                Err(e) => return Outcome::from_error(&e),
            },
        };
        let booking = BookDocumentAmount {
            amount: eur(signed),
            date: OffsetDateTime::now_utc(),
            booking_type: BookingType::FullPayment,
            check_account: ObjectRef::new(account(ZEIT)),
            check_account_transaction: ObjectRef::new(tx),
        };
        let r = match doc.kind {
            Kind::Invoice => {
                self.ctx
                    .c
                    .book_invoice_amount(InvoiceId::new(doc.id), &booking)
                    .await
            }
            Kind::CreditNote => {
                self.ctx
                    .c
                    .book_credit_note_amount(CreditNoteId::new(doc.id), &booking)
                    .await
            }
        };
        match r {
            Ok(()) => {
                doc.tx = Some(tx);
                Outcome::ok("booked")
            }
            Err(e) => {
                doc.spare = Some(tx);
                Outcome::from_error(&e)
            }
        }
    }

    pub(crate) async fn run_op(&self, doc: &mut Doc, op: Op) -> Outcome {
        let obj = doc.kind.obj();
        let path = format!("/{obj}/{}", doc.id);
        let (inv, cn) = (InvoiceId::new(doc.id), CreditNoteId::new(doc.id));
        let c = &self.ctx.c;
        match op {
            Op::PdfPrevent => Self::outcome_of(match doc.kind {
                Kind::Invoice => c.invoice_pdf(inv).await.map(|p| pdf_summary(&p)),
                Kind::CreditNote => c.credit_note_pdf(cn).await.map(|p| pdf_summary(&p)),
            }),
            Op::Xml => Self::outcome_of(
                match doc.kind {
                    Kind::Invoice => c.invoice_xml(inv).await,
                    Kind::CreditNote => c.credit_note_xml(cn).await,
                }
                .map(|x| format!("{} bytes of xml", x.len())),
            ),
            Op::ContentPut => {
                let body = HeaderBody {
                    header: format!("{}-PUT-CHANGED", self.ctx.marker),
                };
                let out = self.raw_put(&path, Some(&body)).await;
                if !out.ok {
                    return out;
                }
                let now = self.read_field(doc.kind, doc.id, "header").await;
                Outcome::ok(format!(
                    "200; header really changed: {}",
                    now.contains("PUT-CHANGED")
                ))
            }
            Op::ContentSave => self.content_save(doc).await,
            Op::ChangeParam => {
                let p = DocumentParameter::Language(DocumentLanguage::EnUs);
                Self::outcome_of(
                    match doc.kind {
                        Kind::Invoice => c.change_invoice_parameter(inv, p).await,
                        Kind::CreditNote => c.change_credit_note_parameter(cn, p).await,
                    }
                    .map(|()| "ok".to_owned()),
                )
            }
            Op::Enshrine => {
                let before = self.read_field(doc.kind, doc.id, "enshrined").await;
                let out = self.enshrine_raw(doc, "op").await;
                let after = self.read_field(doc.kind, doc.id, "enshrined").await;
                Outcome {
                    message: format!("{}; enshrined before {before} after {after}", out.message),
                    ..out
                }
            }
            Op::PdfNoPrevent => {
                let (s, t) = self
                    .ctx
                    .raw(Method::GET, &format!("{path}/getPdf"), None, None)
                    .await;
                Outcome::from_raw(s, &t)
            }
            Op::CreditNoteRef => {
                let note = format!("{}-REF-{}", self.ctx.marker, doc.id);
                let number = match c.next_credit_note_number().await {
                    Ok(n) => n,
                    Err(e) => return Outcome::from_error(&e),
                };
                match c
                    .save_credit_note(&self.credit_note_save(number, note, inv))
                    .await
                {
                    Ok(saved) => {
                        self.created.set(self.created.get() + 1);
                        // A credit note beside the invoice would change later
                        // answers (code 170): take it away again.
                        if c.delete_credit_note(saved.credit_note.id).await.is_err() {
                            doc.dirty = true;
                        }
                        Outcome::ok(format!("draft credit note {} saved", saved.credit_note.id))
                    }
                    Err(e) => Outcome::from_error(&e),
                }
            }
            Op::ResetOpen => Self::outcome_of(
                match doc.kind {
                    Kind::Invoice => c.reset_invoice_to_open(inv).await,
                    Kind::CreditNote => c.reset_credit_note_to_open(cn).await,
                }
                .map(|()| "ok".to_owned()),
            ),
            Op::ResetDraft => {
                let (s, t) = self
                    .ctx
                    .raw(Method::PUT, &format!("{path}/resetToDraft"), None, None)
                    .await;
                Outcome::from_raw(s, &t)
            }
            Op::Cancel => match c.cancel_invoice(inv).await {
                Ok(i) => {
                    self.created.set(self.created.get() + 1);
                    // The cancellation invoice changes later answers (code 170)
                    // without changing the state key.
                    doc.dirty = true;
                    Outcome::ok(format!("cancellation invoice {} ({:?})", i.id, i.status))
                }
                Err(e) => Outcome::from_error(&e),
            },
            Op::Book | Op::Book2 => self.book(doc).await,
            Op::Delete => Self::outcome_of(
                match doc.kind {
                    Kind::Invoice => c.delete_invoice(inv).await,
                    Kind::CreditNote => c.delete_credit_note(cn).await,
                }
                .map(|()| "ok".to_owned()),
            ),
            Op::DeleteTx => match doc.tx {
                Some(tx) => {
                    Self::outcome_of(c.delete_transaction(tx).await.map(|()| "ok".to_owned()))
                }
                None => Outcome::ok("no transaction (skipped)"),
            },
        }
    }

    /// The documented way to update a draft, tried on every state: save with
    /// the document's id and a changed header.
    async fn content_save(&self, doc: &Doc) -> Outcome {
        let note = format!("{}-SAVE-CHANGED", self.ctx.marker);
        let (mut body, key, path) = match doc.kind {
            Kind::Invoice => (
                serde_json::to_value(self.invoice_save(note, 1)).unwrap(),
                "invoice",
                "/Invoice/Factory/saveInvoice",
            ),
            Kind::CreditNote => (
                serde_json::to_value(
                    self.credit_note_save(
                        self.ctx
                            .c
                            .next_credit_note_number()
                            .await
                            .unwrap_or_default(),
                        note,
                        self.origin,
                    ),
                )
                .unwrap(),
                "creditNote",
                "/CreditNote/Factory/saveCreditNote",
            ),
        };
        body[key]["id"] = doc.id.into();
        let (s, t) = self
            .ctx
            .raw(Method::POST, path, Some(body.to_string()), None)
            .await;
        Outcome::from_raw(s, &t)
    }
}
