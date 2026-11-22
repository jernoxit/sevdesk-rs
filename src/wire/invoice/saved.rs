use super::{Invoice, InvoicePosition};
use crate::wire::Guard;
use serde::Deserialize;

/// The answer of `saveInvoice`: the invoice and ALL its positions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedInvoice {
    pub invoice: Invoice,
    /// The positions as saved; present in every recording, so a missing list reads empty.
    #[serde(default)]
    pub invoice_pos: Vec<InvoicePosition>,
    /// Measured as `"not relevant"`.
    #[serde(default)]
    pub vatdrop: Option<String>,
    /// Form not measured; type from the OpenAPI spec (a request artifact there).
    #[serde(default)]
    pub filename: Option<String>,
}

impl Guard for SavedInvoice {
    const NAME: &'static str = "saveInvoice";
    fn load_bearing_present(&self) -> bool {
        self.invoice.load_bearing_present()
    }
}
