use crate::codec::{amount_opt, decimal_opt, deserialize_flag_opt};
use crate::ids::*;
use crate::status::*;
use crate::wire::Guard;
use crate::{Amount, CurrencyCode, Decimal};
use serde::Deserialize;
use time::OffsetDateTime;

/// A Voucher (expense document).
///
/// Every key a recording or the shape survey shows filled is a field. The
/// keys only the OpenAPI spec names say so in their doc; the keys no
/// measurement fills and the spec does not type (`accountingSpecialCase`,
/// `additionalInformation`, `iban`, `inSource`, `propertyCreationOrigin`,
/// `propertyIsBookkeepingV1`, `recurringIntervall`, `resultDisdar`,
/// `sendType`, `taxmaroStockAccount`, `vatNumber`) are not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Voucher {
    pub id: VoucherId,
    /// Form not measured; type from the OpenAPI spec (a request artifact there).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub map_all: Option<bool>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The sevDesk client (the account's own company) the voucher belongs to.
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    /// The sevDesk user who created the voucher.
    #[serde(default)]
    pub create_user: Option<ObjectRef<SevUserId>>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub voucher_date: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date: Option<OffsetDateTime>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date_until: Option<OffsetDateTime>,
    #[serde(default)]
    pub supplier_name: Option<String>,
    /// The supplier name as it was when the voucher was saved; `null` on most
    /// vouchers. Not in the OpenAPI spec.
    #[serde(default)]
    pub supplier_name_at_save: Option<String>,
    /// The supplier contact. In no recording or survey object; the form is
    /// from the OpenAPI spec.
    #[serde(default)]
    pub supplier: Option<ObjectRef<ContactId>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<VoucherStatus>,
    /// When the voucher was paid in full; `None` while it is not.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub pay_date: Option<OffsetDateTime>,
    pub currency: CurrencyCode,
    #[serde(default)]
    pub credit_debit: Option<CreditDebit>,
    #[serde(default)]
    pub voucher_type: Option<VoucherType>,
    #[serde(default)]
    pub tax_rule: Option<ObjectRef<TaxRuleId>>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub tax_type: Option<TaxType>,
    /// The attached PDF, if any; `saveVoucher` embeds the full object at
    /// [`SavedVoucher::document`](super::SavedVoucher::document).
    #[serde(default)]
    pub document: Option<ObjectRef<DocumentId>>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub cost_centre: Option<ObjectRef<CostCentreId>>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub tax_set: Option<ObjectRef<TaxSetId>>,
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
    pub sum_discount_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_gross_foreign_currency: Option<Amount>,
    /// Form not measured; type from the OpenAPI spec (read-only there).
    #[serde(default)]
    pub sum_discounts: Option<String>,
    /// Form not measured; type from the OpenAPI spec (read-only there).
    #[serde(default)]
    pub sum_discounts_foreign_currency: Option<String>,
    /// Positive when an expense is paid (the booking amount carries the
    /// transaction's negative sign; the voucher reports the magnitude).
    #[serde(default, deserialize_with = "amount_opt")]
    pub paid_amount: Option<Amount>,
    /// Net display switch; the wire has `"0"`/`"1"` (`"1"` measured only).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub show_net: Option<bool>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub hidden: Option<bool>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub selected_for_payment_file: Option<bool>,
    /// Tip; measured `"0"` only.
    #[serde(default, deserialize_with = "amount_opt")]
    pub tip: Option<Amount>,
    /// Mileage rate per kilometre, a unit price: kept exact; measured `"0"` only.
    #[serde(default, deserialize_with = "decimal_opt")]
    pub mileage_rate: Option<Decimal>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub enshrined: Option<OffsetDateTime>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub payment_deadline: Option<OffsetDateTime>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub recurring_interval: Option<RecurringInterval>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub recurring_start_date: Option<OffsetDateTime>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub recurring_next_voucher: Option<OffsetDateTime>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub recurring_last_voucher: Option<OffsetDateTime>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub recurring_end_date: Option<OffsetDateTime>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub property_foreign_currency_deadline: Option<OffsetDateTime>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub property_exchange_rate: Option<String>,
}

impl Guard for Voucher {
    const NAME: &'static str = "Voucher";
    fn load_bearing_present(&self) -> bool {
        self.status.is_some() || self.sum_gross.is_some()
    }
}
