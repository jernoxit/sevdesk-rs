//! The calls of the probe, each answering a [`Res`]: the outcome, the id of
//! the document it names and the number in the answer.
use crate::docs::raw_one;
use crate::harness::Ctx;
use crate::record::Outcome;
use reqwest::Method;
use serde::Serialize;
use sevdesk::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct Res {
    pub(crate) out: Outcome,
    pub(crate) id: Option<i64>,
    /// The document number as the answer carries it (`-` when none).
    pub(crate) number: String,
}

fn res_of<T>(r: Result<T, SevdeskError>, pick: impl FnOnce(&T) -> (Option<i64>, String)) -> Res {
    match r {
        Ok(v) => {
            let (id, number) = pick(&v);
            Res {
                out: Outcome::ok("ok"),
                id,
                number,
            }
        }
        Err(e) => Res {
            out: Outcome::from_error(&e),
            id: None,
            number: "-".into(),
        },
    }
}

fn no_answer(_: &()) -> (Option<i64>, String) {
    (None, "-".into())
}

fn cn_pick(c: &CreditNote) -> (Option<i64>, String) {
    (
        Some(c.id.get()),
        c.credit_note_number.clone().unwrap_or_else(|| "-".into()),
    )
}

fn inv_pick(i: &Invoice) -> (Option<i64>, String) {
    (
        Some(i.id.get()),
        i.invoice_number.clone().unwrap_or_else(|| "-".into()),
    )
}

pub(crate) async fn cn_create(ctx: Arc<Ctx>, save: SaveCreditNote) -> Res {
    res_of(ctx.c.save_credit_note(&save).await, |s| {
        cn_pick(&s.credit_note)
    })
}

pub(crate) async fn cn_finalize(ctx: Arc<Ctx>, id: i64) -> Res {
    res_of(
        ctx.c
            .send_credit_note_by(CreditNoteId::new(id), &SendBy::FINALIZE)
            .await,
        cn_pick,
    )
}

pub(crate) async fn cn_enshrine(ctx: Arc<Ctx>, id: i64) -> Res {
    res_of(
        ctx.c.enshrine_credit_note(CreditNoteId::new(id)).await,
        no_answer,
    )
}

pub(crate) async fn inv_finalize(ctx: Arc<Ctx>, id: i64) -> Res {
    res_of(
        ctx.c
            .send_invoice_by(InvoiceId::new(id), &SendBy::FINALIZE)
            .await,
        inv_pick,
    )
}

pub(crate) async fn inv_enshrine(ctx: Arc<Ctx>, id: i64) -> Res {
    res_of(ctx.c.enshrine_invoice(InvoiceId::new(id)).await, no_answer)
}

/// The text field `field` of a raw answer's object, `-` when absent.
fn raw_field(text: &str, field: &str) -> String {
    raw_one(text)[field]
        .as_str()
        .map_or_else(|| "-".to_owned(), str::to_owned)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceNumberBody {
    invoice_number: String,
}

/// `PUT /Invoice/{id}` with only `invoiceNumber`: the invoice draft type has
/// no number field, so the preset number goes through this raw call.
pub(crate) async fn inv_set_number(ctx: Arc<Ctx>, id: i64, number: String) -> Res {
    let body = serde_json::to_string(&InvoiceNumberBody {
        invoice_number: number,
    })
    .unwrap();
    let (status, text) = ctx
        .raw(Method::PUT, &format!("/Invoice/{id}"), Some(body), None)
        .await;
    let out = Outcome::from_raw(status, &text);
    let number = if out.ok {
        raw_field(&text, "invoiceNumber")
    } else {
        "-".into()
    };
    Res {
        out,
        id: Some(id),
        number,
    }
}

#[derive(Serialize)]
struct ObjectName {
    id: i64,
    #[serde(rename = "objectName")]
    object_name: &'static str,
}

#[derive(Serialize)]
struct CreateFromInvoiceBody {
    invoice: ObjectName,
}

/// `POST /CreditNote/Factory/createFromInvoice`, body
/// `{"invoice": {"id": …, "objectName": "Invoice"}}`. The id of the new credit
/// note is read from `creditNote.id` or `id` of the answer.
pub(crate) async fn cn_create_from_invoice(ctx: Arc<Ctx>, invoice: i64) -> Res {
    let body = serde_json::to_string(&CreateFromInvoiceBody {
        invoice: ObjectName {
            id: invoice,
            object_name: "Invoice",
        },
    })
    .unwrap();
    let (status, text) = ctx
        .raw(
            Method::POST,
            "/CreditNote/Factory/createFromInvoice",
            Some(body),
            Some("number_probe_createFromInvoice"),
        )
        .await;
    let out = Outcome::from_raw(status, &text);
    if !out.ok {
        return Res {
            out,
            id: None,
            number: "-".into(),
        };
    }
    let v = raw_one(&text);
    let note = if v["creditNote"].is_object() {
        &v["creditNote"]
    } else {
        &v
    };
    let id = note["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| note["id"].as_i64());
    Res {
        out,
        id,
        number: note["creditNoteNumber"]
            .as_str()
            .map_or_else(|| "-".to_owned(), str::to_owned),
    }
}
