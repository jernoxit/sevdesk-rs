use crate::Amount;
use crate::codec::amount_opt;
use crate::ids::*;
use crate::status::VoucherStatus;
use crate::wire::Guard;
use serde::Deserialize;
use time::OffsetDateTime;

/// The log entry of a booking, the answer of `bookAmount`.
///
/// Carries no currency: `amount_payed` is in the voucher's. `additionalInformation`
/// is `null` in both recordings and not in the OpenAPI spec, so it is not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherLog {
    pub id: VoucherLogId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    /// The booked voucher.
    #[serde(default)]
    pub voucher: Option<ObjectRef<VoucherId>>,
    /// The voucher's status before the booking.
    #[serde(default)]
    pub from_status: Option<VoucherStatus>,
    /// The voucher's status after the booking.
    #[serde(default)]
    pub to_status: Option<VoucherStatus>,
    /// The amount booked; measured positive (`"0.45"`).
    #[serde(default, deserialize_with = "amount_opt")]
    pub amount_payed: Option<Amount>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub booking_date: Option<OffsetDateTime>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
}

impl Guard for VoucherLog {
    const NAME: &'static str = "VoucherLog";
    fn load_bearing_present(&self) -> bool {
        self.to_status.is_some() || self.amount_payed.is_some()
    }
}
