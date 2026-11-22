//! The credit-note resource group: the credit note, its positions and the
//! `saveCreditNote` answer, one file each.

mod document;
mod position;
mod saved;

pub use document::CreditNote;
pub use position::CreditNotePosition;
pub use saved::SavedCreditNote;
