use super::{CreditNote, CreditNotePosition};
use crate::wire::Guard;
use serde::Deserialize;

/// The answer of `saveCreditNote`: the credit note and ALL its positions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCreditNote {
    pub credit_note: CreditNote,
    /// The positions as saved; present in every recording, so a missing list reads empty.
    #[serde(default)]
    pub credit_note_pos: Vec<CreditNotePosition>,
}

impl Guard for SavedCreditNote {
    const NAME: &'static str = "saveCreditNote";
    fn load_bearing_present(&self) -> bool {
        self.credit_note.load_bearing_present()
    }
}
