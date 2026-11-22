//! The credit-note resource group: the credit note, its positions, the
//! `saveCreditNote` answer and the booking log entry, one file each.

mod document;
mod log;
mod position;
mod saved;

pub use document::CreditNote;
pub use log::CreditNoteLog;
pub use position::CreditNotePosition;
pub use saved::SavedCreditNote;
