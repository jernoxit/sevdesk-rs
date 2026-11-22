use crate::codec::{
    amount_opt, decimal_opt, deserialize_flag_opt, deserialize_i64_opt, percent_bp_opt,
    quantity_opt,
};
use crate::ids::*;
use crate::{Amount, Decimal};
use serde::Deserialize;
use time::OffsetDateTime;

/// One position of a credit note (`creditNotePos` of the `saveCreditNote`
/// answer, `GET /CreditNotePos`).
///
/// Every number comes as a string. Not read: `additionalInformation` and
/// `createNextPart` (`null` in every recording and survey object, and not in
/// the spec).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditNotePosition {
    pub id: CreditNotePositionId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The credit note. Only the id is read.
    #[serde(default)]
    pub credit_note: Option<ObjectRef<CreditNoteId>>,
    /// Form not measured; type from the OpenAPI spec.
    #[serde(default)]
    pub part: Option<ObjectRef<PartId>>,
    /// Measured as a string (`"1"`, `"20"`).
    #[serde(default, deserialize_with = "quantity_opt")]
    pub quantity: Option<Decimal>,
    /// Unit price, up to two fraction digits measured (`"10"`).
    #[serde(default, deserialize_with = "decimal_opt")]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub priority: Option<i64>,
    /// The unit. Only the id is read.
    #[serde(default)]
    pub unity: Option<ObjectRef<UnityId>>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub position_number: Option<i64>,
    /// Free text below the position; `null` on all but three surveyed positions.
    #[serde(default)]
    pub text: Option<String>,
    /// The discount as an exact decimal: `is_percentage` decides whether it is
    /// a percent (`"10"` is 10 %) or a money amount.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub discount: Option<Decimal>,
    /// `true` when `discount` is a percentage; the wire has `"1"`.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_percentage: Option<bool>,
    /// Measured `"10"` once, else `null`; like `discount`, `is_percentage`
    /// decides whether it is a percent or an amount.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub discounted_value: Option<Decimal>,
    /// Tax rate in basis points (`"19"` is `1900`).
    #[serde(default, deserialize_with = "percent_bp_opt")]
    pub tax_rate: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub temporary: Option<bool>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_accounting: Option<Amount>,
    /// Net unit price, up to eight fraction digits measured in the survey.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub price_net: Option<Decimal>,
    /// Gross unit price, up to two fraction digits measured.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub price_gross: Option<Decimal>,
    /// Unit price tax, up to eight fraction digits.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub price_tax: Option<Decimal>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_foreign_currency: Option<Amount>,
    /// Form not measured; type from the OpenAPI spec.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub optional: Option<bool>,
}
