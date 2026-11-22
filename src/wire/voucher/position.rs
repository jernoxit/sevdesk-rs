use crate::codec::{amount_opt, decimal_opt, deserialize_flag_opt, percent_bp_opt};
use crate::ids::*;
use crate::wire::Guard;
use crate::{Amount, Decimal};
use serde::Deserialize;
use time::OffsetDateTime;

/// A VoucherPos. Carries no currency: its sums are in its voucher's.
///
/// Every key a recording or the shape survey shows filled is a field. The
/// keys only the OpenAPI spec names say so in their doc; the keys no
/// measurement fills and the spec does not type (`additionalInformation`,
/// `cateringTaxRate`, `createNextPart`, `specialAccountingField1` to
/// `specialAccountingField5`, `sum`) are not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherPosition {
    pub id: VoucherPositionId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default)]
    pub voucher: Option<ObjectRef<VoucherId>>,
    #[serde(default)]
    pub account_datev: Option<ObjectRef<AccountDatevId>>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub accounting_type: Option<ObjectRef<AccountingTypeId>>,
    /// In no recording or survey object; the form is from the OpenAPI spec
    /// (read-only there).
    #[serde(default)]
    pub estimated_accounting_type: Option<ObjectRef<AccountingTypeId>>,
    /// Basis points.
    #[serde(default, rename = "taxRate", deserialize_with = "percent_bp_opt")]
    pub tax_rate_bp: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub net: Option<bool>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_asset: Option<bool>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_gwg: Option<bool>,
    /// Measured `"1"` only; decides whether `discounted_value` is a percent or an amount.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_percentage: Option<bool>,
    /// Measured `"0"` only; like the discount of a document, a percent or an
    /// amount as `is_percentage` says, so it stays an exact decimal.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub discounted_value: Option<Decimal>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub asset_memo_value: Option<Amount>,
    /// Measured `"0"` on some positions, else `null`.
    #[serde(default, deserialize_with = "amount_opt")]
    pub catering_tip: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_foreign_currency: Option<Amount>,
    #[serde(default)]
    pub comment: Option<String>,
}

impl Guard for VoucherPosition {
    const NAME: &'static str = "VoucherPos";
    fn load_bearing_present(&self) -> bool {
        self.sum_net.is_some() || self.sum_gross.is_some()
    }
}
