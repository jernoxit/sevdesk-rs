use super::Guard;
use crate::ids::*;
use serde::Deserialize;

/// A tax rule a booking account allows.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedTaxRule {
    pub id: TaxRuleId,
    #[serde(default)]
    pub name: Option<String>,
    /// `ZERO`, `SEVEN`, `NINETEEN`, …
    #[serde(default)]
    pub tax_rates: Vec<String>,
}

/// One entry of `GET /ReceiptGuidance/forExpense`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptGuidance {
    pub account_datev_id: AccountDatevId,
    /// The account number in the chart of accounts, e.g. `"4970"`.
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub account_name: Option<String>,
    #[serde(default)]
    pub allowed_tax_rules: Vec<AllowedTaxRule>,
}

impl Guard for ReceiptGuidance {
    const NAME: &'static str = "ReceiptGuidance";
    fn load_bearing_present(&self) -> bool {
        self.account_number.is_some() || !self.allowed_tax_rules.is_empty()
    }
}

/// The answer of `uploadTempFile`: attach it to a voucher by `filename`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFile {
    pub filename: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub origin_mime_type: Option<String>,
    #[serde(default, deserialize_with = "crate::codec::deserialize_i64_opt")]
    pub pages: Option<i64>,
}

impl Guard for UploadedFile {
    const NAME: &'static str = "uploadTempFile";
    fn load_bearing_present(&self) -> bool {
        !self.filename.is_empty()
    }
}

/// A SevUser (the account's user; a document names one as contact person).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SevUser {
    pub id: SevUserId,
    #[serde(default)]
    pub username: Option<String>,
}

impl Guard for SevUser {
    const NAME: &'static str = "SevUser";
    fn load_bearing_present(&self) -> bool {
        self.username.is_some()
    }
}

/// A number sequence (`GET /SevSequence/Factory/getByType`): `format` holds
/// `%NUMBER` where `next_sequence` goes (`GU-%NUMBER`, `1011`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SevSequence {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub next_sequence: Option<String>,
}

impl SevSequence {
    /// The number the next document of this sequence would get (`GU-1011`).
    pub fn next_number(&self) -> Option<String> {
        let (format, next) = (self.format.as_deref()?, self.next_sequence.as_deref()?);
        format
            .contains("%NUMBER")
            .then(|| format.replace("%NUMBER", next))
    }
}

impl Guard for SevSequence {
    const NAME: &'static str = "SevSequence";
    fn load_bearing_present(&self) -> bool {
        self.format.is_some() && self.next_sequence.is_some()
    }
}

/// A payment method of the account (`GET /PaymentMethod`). The ids are
/// account-specific; `electronic_invoice_id` is the XML code for e-invoices
/// (`68` for "Online-Zahlung"). `translationCode` is unreliable and not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    pub id: PaymentMethodId,
    #[serde(default)]
    pub name: Option<String>,
    /// Comes as a string (`"68"`).
    #[serde(default, deserialize_with = "crate::codec::deserialize_i64_opt")]
    pub electronic_invoice_id: Option<i64>,
}

impl Guard for PaymentMethod {
    const NAME: &'static str = "PaymentMethod";
    fn load_bearing_present(&self) -> bool {
        self.name.is_some() || self.electronic_invoice_id.is_some()
    }
}
