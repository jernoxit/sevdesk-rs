use crate::Amount;
use crate::codec::amount_opt;
use crate::ids::*;
use crate::status::CreditNoteStatus;
use crate::wire::Guard;
use serde::Deserialize;
use time::OffsetDateTime;

/// The log entry of a booking, the answer of `bookAmount` on a credit note.
///
/// Carries no currency: `amount_payed` is in the credit note's. `additionalInformation`
/// is `null` in the recording and not in the OpenAPI spec, so it is not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CreditNoteLog {
    pub id: CreditNoteLogId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    /// The booked credit note.
    #[serde(default)]
    pub credit_note: Option<ObjectRef<CreditNoteId>>,
    /// The credit note's status before the booking.
    #[serde(default)]
    pub from_status: Option<CreditNoteStatus>,
    /// The credit note's status after the booking.
    #[serde(default)]
    pub to_status: Option<CreditNoteStatus>,
    /// The amount booked; measured negative (`"-5.95"`), the sign of the
    /// transaction. sevDesk spells the key `ammountPayed` here (a typo; the voucher's log has `amountPayed`).
    #[serde(default, rename = "ammountPayed", deserialize_with = "amount_opt")]
    pub amount_payed: Option<Amount>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub booking_date: Option<OffsetDateTime>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
}

impl Guard for CreditNoteLog {
    const NAME: &'static str = "CreditNoteLog";
    fn load_bearing_present(&self) -> bool {
        self.to_status.is_some() || self.amount_payed.is_some()
    }
}
