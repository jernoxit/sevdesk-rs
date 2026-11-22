use super::Guard;
use crate::codec::{amount_opt, deserialize_flag_opt, deserialize_i64_opt};
use crate::ids::*;
use crate::status::*;
use crate::{Amount, CurrencyCode};
use serde::Deserialize;
use time::OffsetDateTime;

/// A CheckAccount (`GET /CheckAccount`).
///
/// Every key a recording or the shape survey shows filled is a field. The
/// keys only the OpenAPI spec names say so in their doc; the keys no
/// measurement fills and the spec does not type (`additionalInformation`,
/// `checkAccId`, `countryCode`, `pin`, `translationCode`) are not read. The
/// spec's `autoMapTransactions` (plural) is a typo for `autoMapTransaction`
/// and is not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAccount {
    pub id: CheckAccountId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The sevDesk client (the account's own company) the account belongs to.
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default)]
    pub name: Option<String>,
    /// sevDesk's `type`; no offline account takes transactions.
    #[serde(default, rename = "type")]
    pub kind: Option<CheckAccountType>,
    /// `null` on an account without file import.
    #[serde(default)]
    pub import_type: Option<CheckAccountImportType>,
    /// The currency of `balance` and of the account's transactions.
    pub currency: CurrencyCode,
    /// Measured `"0"` and `"1"`.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub default_account: Option<bool>,
    #[serde(default)]
    pub status: Option<CheckAccountStatus>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub balance: Option<Amount>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub bank_server: Option<String>,
    /// `autoMapTransaction` — SINGULAR; the spec's plural is silently ignored.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub auto_map_transaction: Option<bool>,
    /// Measured `"0"` and `"1"`.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub auto_sync_transactions: Option<bool>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_sync: Option<OffsetDateTime>,
    /// The booking account, e.g. `1201`.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub accounting_number: Option<i64>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub iban: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub bic: Option<String>,
    /// Measured `"0"` and `"1"`.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub base_account: Option<i64>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub priority: Option<i64>,
}

impl Guard for CheckAccount {
    const NAME: &'static str = "CheckAccount";
    fn load_bearing_present(&self) -> bool {
        self.name.is_some() || self.kind.is_some()
    }
}
