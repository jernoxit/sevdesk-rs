//! The position payload shared by invoices and credit notes.
//!
//! Measured on the test account (2026-09-29): amounts go out as exact decimal
//! strings; a negative quantity is accepted (a discount line).

use crate::Amount;
use crate::codec::serialize_amount;
use crate::ids::*;
use crate::payload::ser_percent_bp;
use serde::Serialize;

/// "Stk" in `Unity`.
pub const PIECE: UnityId = UnityId::new(1);

/// One position of an invoice or a credit note; every value is the caller's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPositionDraft {
    /// Whole units; negative for a discount line (measured).
    pub quantity: i64,
    /// The unit price, gross when the document has `show_net: false`.
    pub price: Amount,
    pub name: String,
    /// Free text under the position name; left out when `None`.
    pub text: Option<String>,
    pub unity: UnityId,
    /// Basis points: `1900` for 19 %.
    pub tax_rate_bp: i64,
}

/// Which document a position belongs to; names the `objectName` on the wire.
#[derive(Debug, Clone, Copy)]
pub(super) enum PositionKind {
    Invoice,
    CreditNote,
}

impl PositionKind {
    const fn object_name(self) -> &'static str {
        match self {
            Self::Invoice => "InvoicePos",
            Self::CreditNote => "CreditNotePos",
        }
    }
}

/// The wire form of a position. A named constructor instead of `From`: the
/// `objectName` depends on the document the position is saved with, which the
/// draft does not know.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PositionWire {
    object_name: &'static str,
    map_all: bool,
    quantity: i64,
    #[serde(serialize_with = "serialize_amount")]
    price: Amount,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    unity: ObjectRef<UnityId>,
    #[serde(rename = "taxRate", serialize_with = "ser_percent_bp")]
    tax_rate_bp: i64,
}

impl PositionWire {
    pub(super) fn new(kind: PositionKind, draft: DocumentPositionDraft) -> Self {
        let DocumentPositionDraft {
            quantity,
            price,
            name,
            text,
            unity,
            tax_rate_bp,
        } = draft;
        Self {
            object_name: kind.object_name(),
            map_all: true,
            quantity,
            price,
            name,
            text,
            unity: ObjectRef::new(unity),
            tax_rate_bp,
        }
    }
}
