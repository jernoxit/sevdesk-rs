use super::Guard;
use crate::codec::amount_opt;
use crate::ids::*;
use crate::status::*;
use crate::{Amount, CurrencyCode};
use serde::Deserialize;
use time::OffsetDateTime;

/// An Invoice; a cancellation invoice (`SR`) points to its original via `origin`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id: InvoiceId,
    #[serde(default)]
    pub invoice_number: Option<String>,
    #[serde(default)]
    pub invoice_type: Option<InvoiceType>,
    #[serde(default)]
    pub status: Option<InvoiceStatus>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub invoice_date: Option<OffsetDateTime>,
    /// Start of the service period.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date: Option<OffsetDateTime>,
    /// End of the service period.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date_until: Option<OffsetDateTime>,
    /// Last change at sevDesk; what `updateAfter` filters on.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// When the invoice was enshrined (finalized); `None` for a draft.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub enshrined: Option<OffsetDateTime>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub paid_amount: Option<Amount>,
    pub currency: CurrencyCode,
    #[serde(default)]
    pub customer_internal_note: Option<String>,
    #[serde(default)]
    pub origin: Option<ObjectRef<InvoiceId>>,
}

impl Invoice {
    /// This is a cancellation invoice (`SR`) whose `origin` is `original`.
    pub fn cancels(&self, original: InvoiceId) -> bool {
        self.invoice_type == Some(InvoiceType::Cancellation)
            && self.origin.is_some_and(|origin| origin.id == original)
    }
}

impl Guard for Invoice {
    const NAME: &'static str = "Invoice";
    fn load_bearing_present(&self) -> bool {
        self.status.is_some() || self.sum_gross.is_some()
    }
}

/// The answer of `saveInvoice`.
#[derive(Debug, Clone, Deserialize)]
pub struct SavedInvoice {
    pub invoice: Invoice,
}

impl Guard for SavedInvoice {
    const NAME: &'static str = "saveInvoice";
    fn load_bearing_present(&self) -> bool {
        self.invoice.load_bearing_present()
    }
}
