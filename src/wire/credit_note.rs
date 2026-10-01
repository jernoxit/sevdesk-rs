use super::Guard;
use crate::codec::amount_opt;
use crate::ids::*;
use crate::status::*;
use crate::{Amount, CurrencyCode};
use serde::Deserialize;
use time::OffsetDateTime;

/// A CreditNote.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditNote {
    pub id: CreditNoteId,
    #[serde(default)]
    pub credit_note_number: Option<String>,
    #[serde(default)]
    pub status: Option<CreditNoteStatus>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub credit_note_date: Option<OffsetDateTime>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub paid_amount: Option<Amount>,
    pub currency: CurrencyCode,
    #[serde(default)]
    pub customer_internal_note: Option<String>,
    /// Set only by `createFromInvoice` (measured); a credit note saved with
    /// `refSrcInvoice` has none.
    #[serde(default)]
    pub origin: Option<ObjectRef<InvoiceId>>,
    /// The invoice this credit note refers to (`refSrcInvoice`).
    #[serde(default)]
    pub ref_src_invoice: Option<ObjectRef<InvoiceId>>,
    /// When the credit note was enshrined; `None` for a draft.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub enshrined: Option<OffsetDateTime>,
}

impl Guard for CreditNote {
    const NAME: &'static str = "CreditNote";
    fn load_bearing_present(&self) -> bool {
        self.status.is_some() || self.sum_gross.is_some()
    }
}

/// The answer of `saveCreditNote`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCreditNote {
    pub credit_note: CreditNote,
}

impl Guard for SavedCreditNote {
    const NAME: &'static str = "saveCreditNote";
    fn load_bearing_present(&self) -> bool {
        self.credit_note.load_bearing_present()
    }
}
