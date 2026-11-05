//! The documents of the matrix and how each state is reached.
#![allow(
    clippy::unwrap_used,
    reason = "test helper: a failure here is a test failure"
)]

use crate::docs::*;
use crate::harness::*;
use crate::record::{Outcome, Recorder, St};
use reqwest::Method;
use sevdesk::*;
use std::sync::Arc;
use time::macros::{date, datetime};

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Invoice,
    CreditNote,
}

impl Kind {
    /// The `objectName`, and the path segment.
    pub(crate) fn obj(self) -> &'static str {
        match self {
            Self::Invoice => "Invoice",
            Self::CreditNote => "CreditNote",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum State {
    Draft,
    /// Status 200, `enshrined` null.
    Finalized,
    /// Status 200, `enshrined` set.
    Enshrined,
    /// Status 1000 on an enshrined document.
    Paid,
    /// Paid, and the booking transaction enshrined as well.
    TxEnshrined,
}

impl State {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Finalized => "finalized",
            Self::Enshrined => "enshrined",
            Self::Paid => "paid",
            Self::TxEnshrined => "tx_enshrined",
        }
    }

    fn matches(self, st: &St) -> bool {
        match self {
            Self::Draft => st.status == "100" && !st.enshrined,
            Self::Finalized => st.status == "200" && !st.enshrined,
            Self::Enshrined => st.status == "200" && st.enshrined,
            Self::Paid | Self::TxEnshrined => st.status == "1000" && st.enshrined,
        }
    }
}

/// Ways to try to leave the draft without enshrining.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Finalizer {
    SendBy,
    SendDraft,
    PutStatus,
    ChangeStatus,
    /// `getPdf` without `preventSendBy`.
    PdfRender,
}

impl Finalizer {
    pub(crate) const ALL: [Self; 5] = [
        Self::SendBy,
        Self::SendDraft,
        Self::PutStatus,
        Self::ChangeStatus,
        Self::PdfRender,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::SendBy => "probe_sendBy_VPDF",
            Self::SendDraft => "probe_sendBy_sendDraft_true",
            Self::PutStatus => "probe_PUT_status_200",
            Self::ChangeStatus => "probe_PUT_changeStatus_200",
            Self::PdfRender => "probe_getPdf_no_preventSendBy",
        }
    }
}

pub(crate) struct Doc {
    pub(crate) kind: Kind,
    pub(crate) id: i64,
    /// Gross sum in cents (positive; a credit note books it negative).
    pub(crate) gross: i64,
    /// The transaction that paid it.
    pub(crate) tx: Option<TransactionId>,
    /// A created, unbooked transaction kept for the next booking attempt.
    pub(crate) spare: Option<TransactionId>,
    /// An operation left something behind that the state key does not show.
    pub(crate) dirty: bool,
    pub(crate) st: St,
}

pub(crate) struct Env {
    pub(crate) ctx: Arc<Ctx>,
    pub(crate) run: String,
    pub(crate) customer: Customer,
    pub(crate) person: SevUserId,
    pub(crate) pm: PaymentMethodId,
    /// An enshrined invoice of this run that credit notes refer to.
    pub(crate) origin: InvoiceId,
    pub(crate) finalizer: [Option<Finalizer>; 2],
    pub(crate) seq: u32,
    pub(crate) created: std::cell::Cell<u32>,
    pub(crate) rec: Recorder,
    streak: (String, u32),
}

impl Env {
    pub(crate) async fn new(run: &str, ctx: Arc<Ctx>) -> Self {
        let (status, text) = ctx.raw(Method::GET, "/SevClient", None, None).await;
        assert!(status < 300, "GET /SevClient answered {status}");
        let id = raw_objects(&text)[0]["id"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());
        assert_eq!(
            id,
            Some(TEST_SEV_CLIENT),
            "not the test account: aborting before any write"
        );
        let methods = ctx.c.list_payment_methods().await.unwrap();
        let pm = methods
            .iter()
            .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
            .expect("a payment method with electronicInvoiceId 68")
            .id;
        let customer = b2b_customer(&ctx, false).await;
        let person = contact_person(&ctx).await;
        let mut env = Self {
            ctx,
            run: run.into(),
            customer,
            person,
            pm,
            origin: InvoiceId::new(0),
            finalizer: [None, None],
            seq: 0,
            created: std::cell::Cell::new(0),
            rec: Recorder::new(run),
            streak: (String::new(), 0),
        };
        let origin = env.new_draft(Kind::Invoice, "origin", 20).await.unwrap();
        env.enshrine_doc(&origin).await.unwrap();
        env.origin = InvoiceId::new(origin.id);
        eprintln!("SETUP origin invoice {}", origin.id);
        env
    }

    /// Counts consecutive identical setup failures; three abort the run.
    pub(crate) fn setup_failed(&mut self, e: &str) {
        if self.streak.0 == e {
            self.streak.1 += 1;
        } else {
            self.streak = (e.to_owned(), 1);
        }
        assert!(
            self.streak.1 < 3,
            "setup failed three times in a row with the same error: {e}"
        );
    }

    pub(crate) fn setup_ok(&mut self) {
        self.streak = (String::new(), 0);
    }

    /// `status`, `enshrined` and the paid amount, read raw.
    pub(crate) async fn read_state(&self, kind: Kind, id: i64) -> St {
        let (status, text) = self
            .ctx
            .raw(Method::GET, &format!("/{}/{id}", kind.obj()), None, None)
            .await;
        if status >= 300 {
            return St {
                status: format!("gone(HTTP {status})"),
                enshrined: false,
                paid: "-".into(),
            };
        }
        let v = raw_one(&text);
        let text_of = |k: &str| match &v[k] {
            serde_json::Value::Null => "-".to_owned(),
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        St {
            status: text_of("status"),
            enshrined: !v["enshrined"].is_null(),
            paid: text_of("paidAmount"),
        }
    }

    /// The raw `enshrined` value (a timestamp string or `null`) and `header`.
    pub(crate) async fn read_field(&self, kind: Kind, id: i64, field: &str) -> String {
        let (status, text) = self
            .ctx
            .raw(Method::GET, &format!("/{}/{id}", kind.obj()), None, None)
            .await;
        if status >= 300 {
            return format!("HTTP {status}");
        }
        raw_one(&text)[field].to_string()
    }

    fn note(&mut self, label: &str) -> String {
        self.seq += 1;
        format!("{}-{:03}-{label}", self.ctx.marker, self.seq)
    }

    fn spec(&self, note: String) -> InvoiceSpec<'_> {
        InvoiceSpec {
            customer: &self.customer,
            person: self.person,
            note,
            b2b: true,
            payment_method: Some(self.pm),
            with_address: true,
        }
    }

    /// The invoice draft of a document, for creating it and for the
    /// content-save probe.
    pub(crate) fn invoice_save(&self, note: String, quantity: i64) -> SaveInvoice {
        SaveInvoice::new(
            invoice_draft(&self.spec(note)),
            vec![net_position(quantity, 1000, "SDRS-ENSH Position", None)],
        )
    }

    pub(crate) fn credit_note_save(
        &self,
        number: String,
        note: String,
        origin: InvoiceId,
    ) -> SaveCreditNote {
        let c = &self.customer;
        SaveCreditNote::new(
            CreditNoteDraft {
                credit_note_number: number,
                contact: ObjectRef::new(c.contact.id),
                contact_person: ObjectRef::new(self.person),
                credit_note_date: date!(2026 - 10 - 01),
                delivery_date: datetime!(2026-10-01 0:00 +2),
                delivery_date_until: Some(datetime!(2026-10-31 0:00 +1)),
                header: note.clone(),
                head_text: None,
                foot_text: None,
                tax_rule: ObjectRef::new(TaxRuleId::new(1)),
                tax_text: "Umsatzsteuer 19%".into(),
                booking_category: CreditNoteCategory::Underachievement,
                ref_src_invoice: Some(ObjectRef::new(origin)),
                currency: CurrencyCode::EUR,
                show_net: true,
                customer_internal_note: note,
                address_name: Some(c.name.clone()),
                address_street: Some(c.street.into()),
                address_zip: Some(c.zip.into()),
                address_city: Some(c.city.into()),
                address_country: Some(ObjectRef::new(GERMANY)),
                payment_method: Some(ObjectRef::new(self.pm)),
                is_e_invoice: true,
            },
            vec![net_position(1, 500, "SDRS-ENSH Position", None)],
        )
    }

    /// A fresh draft; `quantity` scales the invoice position.
    pub(crate) async fn new_draft(
        &mut self,
        kind: Kind,
        label: &str,
        quantity: i64,
    ) -> Result<Doc, String> {
        let note = self.note(label);
        let (id, gross) = match kind {
            Kind::Invoice => {
                let saved = self
                    .ctx
                    .c
                    .save_invoice(&self.invoice_save(note, quantity))
                    .await
                    .map_err(|e| e.to_string())?;
                (saved.invoice.id.get(), saved.invoice.sum_gross)
            }
            Kind::CreditNote => {
                let number = self
                    .ctx
                    .c
                    .next_credit_note_number()
                    .await
                    .map_err(|e| e.to_string())?;
                let save = self.credit_note_save(number, note, self.origin);
                let saved = self
                    .ctx
                    .c
                    .save_credit_note(&save)
                    .await
                    .map_err(|e| e.to_string())?;
                (saved.credit_note.id.get(), saved.credit_note.sum_gross)
            }
        };
        self.created.set(self.created.get() + 1);
        let gross = gross.map_or(1190 * quantity, Amount::minor);
        eprintln!("CREATED {} {id} ({label})", kind.obj());
        Ok(Doc {
            kind,
            id,
            gross,
            tx: None,
            spare: None,
            dirty: false,
            st: St {
                status: "100".into(),
                enshrined: false,
                paid: "-".into(),
            },
        })
    }

    /// `sendBy` with `SendBy::ENSHRINE`, then `PUT …/enshrine` when `sendBy`
    /// alone left the document unenshrined (setting OFF).
    pub(crate) async fn enshrine_doc(&self, doc: &Doc) -> Result<(), String> {
        let o = self.finalize(doc, Finalizer::SendBy).await;
        if !o.ok {
            return Err(format!("sendBy: {}", o.message));
        }
        if !self.read_state(doc.kind, doc.id).await.enshrined {
            let o = self.enshrine_raw(doc, "setup").await;
            if !o.ok {
                return Err(format!("enshrine: {}", o.message));
            }
        }
        Ok(())
    }

    /// Builds a document in `state`, verified against a re-read.
    pub(crate) async fn build(&mut self, kind: Kind, state: State) -> Result<Doc, String> {
        let mut doc = self.new_draft(kind, state.name(), 1).await?;
        match state {
            State::Draft => {}
            State::Finalized => {
                let how =
                    self.finalizer[kind as usize].ok_or("finalized state is not reachable")?;
                let o = self.finalize(&doc, how).await;
                if !o.ok {
                    return Err(format!("finalize: {}", o.message));
                }
            }
            State::Enshrined | State::Paid | State::TxEnshrined => {
                self.enshrine_doc(&doc).await?;
            }
        }
        if matches!(state, State::Paid | State::TxEnshrined) {
            let o = self.book(&mut doc).await;
            if !o.ok {
                return Err(format!("book: {}", o.message));
            }
        }
        if state == State::TxEnshrined {
            let tx = doc.tx.ok_or("no transaction")?;
            self.ctx
                .c
                .enshrine_transaction(tx)
                .await
                .map_err(|e| format!("enshrine transaction: {e}"))?;
        }
        doc.st = self.read_state(kind, doc.id).await;
        if !state.matches(&doc.st) {
            return Err(format!("built {} but state reads {}", state.name(), doc.st));
        }
        Ok(doc)
    }

    pub(crate) fn outcome_of(r: Result<String, SevdeskError>) -> Outcome {
        match r {
            Ok(m) => Outcome::ok(m),
            Err(e) => Outcome::from_error(&e),
        }
    }
}
