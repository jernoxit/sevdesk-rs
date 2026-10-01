use super::Guard;
use crate::codec::{amount_opt, deserialize_flag_opt};
use crate::ids::*;
use crate::{Amount, CurrencyCode};
use serde::Deserialize;

/// A CheckAccount (`GET /CheckAccount`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAccount {
    pub id: CheckAccountId,
    #[serde(default)]
    pub name: Option<String>,
    /// `online` or `offline` (sevDesk's `type`); no offline account takes transactions.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// The currency of `balance` and of the account's transactions.
    pub currency: CurrencyCode,
    /// The booking account, e.g. `1201`.
    #[serde(default, deserialize_with = "crate::codec::deserialize_i64_opt")]
    pub accounting_number: Option<i64>,
    /// `autoMapTransaction` — SINGULAR; the spec's plural is silently ignored.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub auto_map_transaction: Option<bool>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub balance: Option<Amount>,
}

impl Guard for CheckAccount {
    const NAME: &'static str = "CheckAccount";
    fn load_bearing_present(&self) -> bool {
        self.name.is_some() || self.kind.is_some()
    }
}
