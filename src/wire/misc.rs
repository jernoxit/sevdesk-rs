use super::Guard;
use crate::codec::{deserialize_flag_opt, deserialize_i64_opt};
use crate::ids::*;
use crate::status::ReceiptType;
use serde::Deserialize;
use time::OffsetDateTime;

/// The `additionalInformation` object some documents carry (`null` on most).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalInformation {
    /// Measured once: `false` on a cancellation invoice. Other keys of the
    /// object are not measured.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub credit_voucher: Option<bool>,
}

/// A tax rule a booking account allows.
///
/// Every key of the recording is a field (plus `description`, which the
/// OpenAPI spec names too). `id` is a JSON number here, a string elsewhere.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedTaxRule {
    pub id: TaxRuleId,
    /// A rule name such as `VORST_ABZUGSF_AUFW`.
    #[serde(default)]
    pub name: Option<String>,
    /// A readable description of the rule.
    #[serde(default)]
    pub description: Option<String>,
    /// The tax rates combinable with the rule: `ZERO`, `SEVEN`, `NINETEEN`
    /// measured. Stays text: the OpenAPI spec gives that example and no
    /// enumeration, and three values do not make a closed set.
    #[serde(default)]
    pub tax_rates: Vec<String>,
}

/// One entry of `GET /ReceiptGuidance/forExpense`. Every key of the recording
/// is a field.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptGuidance {
    pub account_datev_id: AccountDatevId,
    /// The account number in the chart of accounts, e.g. `"4970"`.
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub account_name: Option<String>,
    /// What the account is used for.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub allowed_tax_rules: Vec<AllowedTaxRule>,
    /// `EXPENSE` and `REVENUE` are measured.
    #[serde(default)]
    pub allowed_receipt_types: Vec<ReceiptType>,
}

impl Guard for ReceiptGuidance {
    const NAME: &'static str = "ReceiptGuidance";
    fn load_bearing_present(&self) -> bool {
        self.account_number.is_some() || !self.allowed_tax_rules.is_empty()
    }
}

/// The answer of `uploadTempFile`: attach it to a voucher by `filename`.
///
/// Every key of the recording is a field except `content`, which is `[]` in
/// the only recording and whose element form no measurement shows (see
/// `FORM_UNMEASURED` of the completeness test). The answer has no `id`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFile {
    pub filename: String,
    /// `image/jpg` for an uploaded PDF (a preview image), measured once.
    #[serde(default)]
    pub mime_type: Option<String>,
    /// The type of the uploaded file itself: `application/pdf` measured.
    #[serde(default)]
    pub origin_mime_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub pages: Option<i64>,
    /// A hash of the file (64 hex digits measured).
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Not in the OpenAPI spec; measured `false`.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_e_invoice: Option<bool>,
}

impl Guard for UploadedFile {
    const NAME: &'static str = "uploadTempFile";
    fn load_bearing_present(&self) -> bool {
        !self.filename.is_empty()
    }
}

/// A SevUser (the account's user; a document names one as contact person).
///
/// `/SevUser` is not in the OpenAPI spec and no recording of the call exists:
/// the fields are the keys of the read-only shape survey, typed by the forms
/// it measured (the survey kept no values, they are personal data). The keys
/// the survey found always `null` (`additionalInformation`, `smtp*`,
/// `startDate`, `endDate`) are not read until a measurement shows them.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SevUser {
    pub id: SevUserId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default)]
    pub fullname: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    /// A code; its meanings are not documented, so it stays a number.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub status: Option<i64>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    /// The user's role as text; no value set is documented, so it stays text.
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub member_code: Option<String>,
    /// The sevDesk client (the account's own company) the user belongs to.
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_login: Option<OffsetDateTime>,
    #[serde(default)]
    pub last_login_ip: Option<String>,
    /// Measured as a string switch (`"0"`/`"1"`).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub welcome_screen_seen: Option<bool>,
    /// A language code as text.
    #[serde(default)]
    pub language_code: Option<String>,
    /// Measured as a string switch.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub two_factor_auth: Option<bool>,
    /// Measured as a string switch.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub force_password_change: Option<bool>,
    /// Measured as a string switch.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub client_owner: Option<bool>,
    /// Measured as a string switch.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub default_receive_mail_copy: Option<bool>,
    /// Measured as a string switch.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub hide_maps_directions: Option<bool>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_password_change: Option<OffsetDateTime>,
}

impl Guard for SevUser {
    const NAME: &'static str = "SevUser";
    fn load_bearing_present(&self) -> bool {
        self.username.is_some()
    }
}

/// A number sequence (`GET /SevSequence/Factory/getByType`): `format` holds
/// `%NUMBER` where `next_sequence` goes (`GU-%NUMBER`, `1011`).
///
/// `/SevSequence` is not in the OpenAPI spec; every key of the live sample is
/// a field except `additionalInformation` (`null` in every measurement, form
/// unmeasured).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SevSequence {
    pub id: SevSequenceId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The object type the sequence numbers: `Invoice`, `CreditNote` measured.
    #[serde(default)]
    pub for_object: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    /// A digit string (`"1011"`), kept as text.
    #[serde(default)]
    pub next_sequence: Option<String>,
    /// sevDesk's `type`: `RE` and `CN` measured (the `type` parameter of the
    /// call). Stays text: no value set is documented.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
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
/// (`68` for "Online-Zahlung").
///
/// `/PaymentMethod` is not in the OpenAPI spec; every key of the recordings
/// is a field except `additionalInformation` (`null` in every measurement,
/// form unmeasured).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    pub id: PaymentMethodId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default)]
    pub name: Option<String>,
    /// The text a document shows for the method.
    #[serde(default)]
    pub text: Option<String>,
    /// Comes as a string (`"68"`).
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub electronic_invoice_id: Option<i64>,
    /// Unreliable: measured `PAYMENT_METHOD_58` on two different methods
    /// (SEPA transfer and SEPA direct debit).
    #[serde(default)]
    pub translation_code: Option<String>,
}

impl Guard for PaymentMethod {
    const NAME: &'static str = "PaymentMethod";
    fn load_bearing_present(&self) -> bool {
        self.name.is_some() || self.electronic_invoice_id.is_some()
    }
}
