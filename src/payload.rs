//! What we send: typed `Serialize` builders, no ad-hoc JSON.
//!
//! Amounts go out as exact decimal strings built from integer cents
//! (`-5749` is `"-57.49"`). Ids go out as integers inside `{id, objectName}`.
//!
//! Confirmed against the test account (`tests/live_write/`): strings are
//! accepted for every amount — also for `taxRate` and `bookAmount` — and
//! `dd.mm.yyyy` voucher dates come back as Berlin midnight
//! (`2026-09-29T00:00:00+02:00`). A voucher is only updatable and deletable
//! as a draft (see [`crate::SevdeskClient::reset_voucher_to_draft`]).

use crate::codec::{serialize_amount, serialize_flag};
use crate::ids::*;
use crate::status::*;
use crate::{Amount, CurrencyCode};
use serde::{Serialize, Serializer};
use time::{Date, OffsetDateTime};

/// `dd.mm.yyyy`, one of the two date forms sevDesk documents for voucher dates
/// (the other is a Unix timestamp).
pub(crate) fn ser_german_date<S: Serializer>(date: &Date, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&format!(
        "{:02}.{:02}.{}",
        date.day(),
        u8::from(date.month()),
        date.year()
    ))
}

fn ser_german_date_opt<S: Serializer>(date: &Option<Date>, s: S) -> Result<S::Ok, S::Error> {
    match date {
        Some(date) => ser_german_date(date, s),
        None => s.serialize_none(),
    }
}

pub(crate) fn ser_unix<S: Serializer>(at: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_i64(at.unix_timestamp())
}

pub(crate) fn ser_unix_opt<S: Serializer>(
    at: &Option<OffsetDateTime>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match at {
        Some(at) => ser_unix(at, s),
        None => s.serialize_none(),
    }
}

/// A tax rate in basis points as sevDesk's decimal string (`1900` is `"19.00"`).
pub(crate) fn ser_percent_bp<S: Serializer>(bp: &i64, s: S) -> Result<S::Ok, S::Error> {
    serialize_amount(&Amount::from_minor(*bp), s)
}

/// The voucher part of a `saveVoucher` call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherDraft {
    object_name: &'static str,
    map_all: bool,
    /// `Some` updates that voucher; `None` creates one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<VoucherId>,
    #[serde(serialize_with = "ser_german_date")]
    pub voucher_date: Date,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    pub delivery_date: Option<Date>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    pub delivery_date_until: Option<Date>,
    /// Free text supplier; left out when empty (a `supplier` contact names it).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub supplier_name: String,
    /// The supplier as a contact of the account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier: Option<ObjectRef<ContactId>>,
    pub description: String,
    pub status: VoucherStatus,
    pub credit_debit: CreditDebit,
    pub voucher_type: VoucherType,
    pub tax_rule: ObjectRef<TaxRuleId>,
    pub currency: CurrencyCode,
}

impl VoucherDraft {
    /// A new draft expense voucher (`status` 50, `creditDebit` C, `voucherType`
    /// VOU); adjust the public fields for anything else. Measured with EUR
    /// only; per spec a currency other than the account's also needs an
    /// exchange rate, which this draft does not send.
    pub fn new(
        voucher_date: Date,
        supplier_name: impl Into<String>,
        description: impl Into<String>,
        tax_rule: TaxRuleId,
        currency: CurrencyCode,
    ) -> Self {
        Self {
            object_name: "Voucher",
            map_all: true,
            id: None,
            voucher_date,
            delivery_date: None,
            delivery_date_until: None,
            supplier_name: supplier_name.into(),
            supplier: None,
            description: description.into(),
            status: VoucherStatus::Draft,
            credit_debit: CreditDebit::Credit,
            voucher_type: VoucherType::Normal,
            tax_rule: ObjectRef::new(tax_rule),
            currency,
        }
    }
}

/// One position of a `saveVoucher` call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherPositionDraft {
    object_name: &'static str,
    map_all: bool,
    /// Present when the position already exists. **On an update every
    /// position must be sent with its id**: sevDesk computes the voucher
    /// total from the positions of THIS call only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<VoucherPositionId>,
    pub account_datev: ObjectRef<AccountDatevId>,
    /// Basis points: `1900` for 19 %.
    #[serde(rename = "taxRate", serialize_with = "ser_percent_bp")]
    pub tax_rate_bp: i64,
    /// `sum_net` is the net amount; always `true`.
    net: bool,
    #[serde(serialize_with = "serialize_amount")]
    pub sum_net: Amount,
    pub comment: String,
}

impl VoucherPositionDraft {
    pub fn new(
        account_datev: AccountDatevId,
        tax_rate_bp: i64,
        sum_net: Amount,
        comment: impl Into<String>,
    ) -> Self {
        Self {
            object_name: "VoucherPos",
            map_all: true,
            id: None,
            account_datev: ObjectRef::new(account_datev),
            tax_rate_bp,
            net: true,
            sum_net,
            comment: comment.into(),
        }
    }

    /// The same position as it already exists at sevDesk.
    pub fn with_id(mut self, id: VoucherPositionId) -> Self {
        self.id = Some(id);
        self
    }
}

/// `POST /Voucher/Factory/saveVoucher`: create or update a draft.
#[derive(Debug, Clone, Serialize)]
pub struct SaveVoucher {
    pub voucher: VoucherDraft,
    #[serde(rename = "voucherPosSave")]
    pub positions: Vec<VoucherPositionDraft>,
    /// The `filename` returned by `uploadTempFile`; attaches the PDF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// `PUT /Voucher/{id}/bookAmount`. Takes no currency: the amount is in the
/// transaction's.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookVoucherAmount {
    /// **Carries the sign of the transaction**: negative for an expense paid
    /// from the account. Positive amounts book without error but leave the
    /// voucher unpaid with a negative `paidAmount`.
    #[serde(serialize_with = "serialize_amount")]
    pub amount: Amount,
    /// A date-time string for vouchers (a Unix timestamp for invoices).
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,
    #[serde(rename = "type")]
    pub booking_type: BookingType,
    pub check_account: ObjectRef<CheckAccountId>,
    pub check_account_transaction: ObjectRef<TransactionId>,
}

/// `PUT /Invoice/{id}/bookAmount` and `PUT /CreditNote/{id}/bookAmount`.
/// Takes no currency: the amount is in the transaction's.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDocumentAmount {
    /// Carries the sign of the transaction (measured: −58.31 pays a credit
    /// note of 58.31).
    #[serde(serialize_with = "serialize_amount")]
    pub amount: Amount,
    /// An integer Unix timestamp for invoices and credit notes.
    #[serde(serialize_with = "ser_unix")]
    pub date: OffsetDateTime,
    #[serde(rename = "type")]
    pub booking_type: BookingType,
    pub check_account: ObjectRef<CheckAccountId>,
    pub check_account_transaction: ObjectRef<TransactionId>,
}

/// `POST /CheckAccountTransaction`. Only online accounts take it (offline: 422).
/// Takes no currency: the amount is in the check account's.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTransaction {
    /// Stored with the offset it is sent with, so the caller chooses the
    /// calendar day (Berlin) by choosing the offset.
    #[serde(with = "time::serde::rfc3339")]
    pub value_date: OffsetDateTime,
    /// `entryDate`, sent in the same format as [`Self::value_date`]. Measured
    /// on the sevDesk test tenant (2026-10-01): it is accepted independently
    /// of `valueDate`, reads back unchanged, and defaults to `valueDate` when
    /// omitted. The payment date of a booking (Invoice/Voucher `payDate`, the
    /// transaction log's `bookingDate`, the DATEV payment line) is the
    /// transaction's `valueDate`, not `entryDate`.
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub entry_date: Option<OffsetDateTime>,
    #[serde(serialize_with = "serialize_amount")]
    pub amount: Amount,
    pub payee_payer_name: String,
    /// Substring-matched by sevDesk's filter; compare exactly after reading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymt_purpose: Option<String>,
    pub status: TransactionStatus,
    pub check_account: ObjectRef<CheckAccountId>,
    /// The "Geldbewegung" link: the transaction this one is rebooked from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_transaction: Option<ObjectRef<TransactionId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_transaction: Option<ObjectRef<TransactionId>>,
}

/// `PUT /CheckAccountTransaction/{id}`. The links are stored only on the
/// side they are set on, so a transfer sets both.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TransactionStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_transaction: Option<ObjectRef<TransactionId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_transaction: Option<ObjectRef<TransactionId>>,
}

/// `PUT /CheckAccount/{id}`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAccountUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `autoMapTransaction`, singular: the spec's `autoMapTransactions` is
    /// silently ignored.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "autoMapTransaction",
        serialize_with = "ser_flag_opt"
    )]
    pub auto_map_transaction: Option<bool>,
}

fn ser_flag_opt<S: Serializer>(v: &Option<bool>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => serialize_flag(v, s),
        None => s.serialize_none(),
    }
}

/// `POST /CheckAccount/Factory/fileImportAccount`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileImportAccount {
    pub name: String,
    pub import_type: ImportType,
    /// Without one, sevDesk assigns the next free booking account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accounting_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iban: Option<String>,
}
