use crate::codec::{
    amount_opt, decimal_opt, deserialize_flag_opt, deserialize_i64_opt, percent_bp_opt,
    quantity_opt, string_opt,
};
use crate::ids::*;
use crate::{Amount, Decimal};
use serde::Deserialize;
use time::OffsetDateTime;

/// One position of an invoice (`invoicePos` of the `saveInvoice` answer,
/// `GET /InvoicePos`).
///
/// The save answer writes numbers as JSON numbers and `quantity` `0`/`1` as
/// `false`/`true`; the GET answer writes strings. Both read the same. Not
/// read: `additionalInformation` (`null` in every recording and not in the
/// spec).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePosition {
    pub id: InvoicePositionId,
    /// `null` in the save answer, filled in the GET answer.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    /// `null` in the save answer, filled in the GET answer.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The invoice. The save answer embeds the full `Invoice`; only the id is read.
    #[serde(default)]
    pub invoice: Option<ObjectRef<InvoiceId>>,
    /// Form not measured; type from the OpenAPI spec.
    #[serde(default)]
    pub part: Option<ObjectRef<PartId>>,
    /// Measured as a number, a string, `"0.5"`, and `false`/`true` for 0/1.
    #[serde(default, deserialize_with = "quantity_opt")]
    pub quantity: Option<Decimal>,
    /// Unit price, up to eight fraction digits (measured `"12.3456"`).
    #[serde(default, deserialize_with = "decimal_opt")]
    pub price: Option<Decimal>,
    /// The position title; an empty string reads as `None`.
    #[serde(default, deserialize_with = "string_opt")]
    pub name: Option<String>,
    /// The unit. The save answer embeds the full `Unity`; only the id is read.
    #[serde(default)]
    pub unity: Option<ObjectRef<UnityId>>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub position_number: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub priority: Option<i64>,
    /// Free text below the position, with line breaks.
    #[serde(default)]
    pub text: Option<String>,
    /// The discount as an exact decimal: `is_percentage` decides whether it is
    /// a percent (`"10"` is 10 %) or a money amount.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub discount: Option<Decimal>,
    /// `true` when `discount` is a percentage. A JSON bool in the save answer,
    /// `"1"` in the GET answer.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_percentage: Option<bool>,
    /// Measured `"10"`, `"20"` on discounted positions, else `null`; like
    /// `discount`, `is_percentage` decides whether it is a percent or an amount.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub discounted_value: Option<Decimal>,
    /// Tax rate in basis points (`"19.00"` is `1900`).
    #[serde(default, deserialize_with = "percent_bp_opt")]
    pub tax_rate: Option<i64>,
    /// `"0"` in the GET answer, `null` in the save answer.
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
    /// Net unit price, up to eight fraction digits (`"8.40336134"`).
    #[serde(default, deserialize_with = "decimal_opt")]
    pub price_net: Option<Decimal>,
    /// Gross unit price, unrounded (`"14.691264"`).
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
    /// `false` in the save answer, `null` in the GET answer.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub create_next_part: Option<bool>,
}
