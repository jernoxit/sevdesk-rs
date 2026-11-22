//! The invoice resource group: the invoice, its positions and the
//! `saveInvoice` answer, one file each.

mod document;
mod position;
mod saved;

pub use document::Invoice;
pub use position::InvoicePosition;
pub use saved::SavedInvoice;
