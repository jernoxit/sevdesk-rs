use super::Guard;
use crate::codec::{amount_opt, deserialize_flag_opt, percent_bp_opt};
use crate::ids::*;
use crate::status::*;
use crate::{Amount, CurrencyCode};
use serde::Deserialize;
use time::OffsetDateTime;

/// A Voucher (expense document).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voucher {
    pub id: VoucherId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub voucher_date: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date_until: Option<OffsetDateTime>,
    #[serde(default)]
    pub supplier_name: Option<String>,
    /// The supplier contact, if one is set.
    #[serde(default)]
    pub supplier: Option<ObjectRef<ContactId>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<VoucherStatus>,
    pub currency: CurrencyCode,
    #[serde(default)]
    pub credit_debit: Option<CreditDebit>,
    #[serde(default)]
    pub voucher_type: Option<VoucherType>,
    #[serde(default)]
    pub tax_rule: Option<ObjectRef<TaxRuleId>>,
    /// The attached PDF, if any.
    #[serde(default)]
    pub document: Option<ObjectRef<DocumentId>>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    /// Positive when an expense is paid (the booking amount carries the
    /// transaction's negative sign; the voucher reports the magnitude).
    #[serde(default, deserialize_with = "amount_opt")]
    pub paid_amount: Option<Amount>,
}

impl Guard for Voucher {
    const NAME: &'static str = "Voucher";
    fn load_bearing_present(&self) -> bool {
        self.status.is_some() || self.sum_gross.is_some()
    }
}

/// A VoucherPos. Carries no currency: its sums are in its voucher's.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherPosition {
    pub id: VoucherPositionId,
    #[serde(default)]
    pub voucher: Option<ObjectRef<VoucherId>>,
    #[serde(default)]
    pub account_datev: Option<ObjectRef<AccountDatevId>>,
    /// Basis points.
    #[serde(default, rename = "taxRate", deserialize_with = "percent_bp_opt")]
    pub tax_rate_bp: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub net: Option<bool>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    #[serde(default)]
    pub comment: Option<String>,
}

impl Guard for VoucherPosition {
    const NAME: &'static str = "VoucherPos";
    fn load_bearing_present(&self) -> bool {
        self.sum_net.is_some() || self.sum_gross.is_some()
    }
}

/// The answer of `saveVoucher`: the voucher and ALL its positions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedVoucher {
    pub voucher: Voucher,
    #[serde(default)]
    pub voucher_pos: Vec<VoucherPosition>,
}

impl Guard for SavedVoucher {
    const NAME: &'static str = "saveVoucher";
    fn load_bearing_present(&self) -> bool {
        self.voucher.load_bearing_present()
    }
}
