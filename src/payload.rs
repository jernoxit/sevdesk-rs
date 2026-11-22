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

use crate::Amount;
use crate::codec::{serialize_amount, serialize_flag};
use crate::ids::*;
use crate::status::*;
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

pub(crate) fn ser_german_date_opt<S: Serializer>(
    date: &Option<Date>,
    s: S,
) -> Result<S::Ok, S::Error> {
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

/// A date as `yyyy-mm-dd` (contact `birthday`).
pub(crate) fn ser_iso_date_opt<S: Serializer>(
    date: &Option<Date>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match date {
        Some(date) => s.serialize_str(&format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            u8::from(date.month()),
            date.day()
        )),
        None => s.serialize_none(),
    }
}

pub(crate) fn ser_percent_bp_opt<S: Serializer>(bp: &Option<i64>, s: S) -> Result<S::Ok, S::Error> {
    match bp {
        Some(bp) => ser_percent_bp(bp, s),
        None => s.serialize_none(),
    }
}

pub(crate) fn ser_amount_opt<S: Serializer>(
    amount: &Option<Amount>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match amount {
        Some(amount) => serialize_amount(amount, s),
        None => s.serialize_none(),
    }
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
/// Takes no currency: the amount is in the check account's. The named fields
/// are required; the rest is in [`Self::options`].
#[derive(Debug, Clone)]
pub struct NewTransaction {
    /// Stored with the offset it is sent with, so the caller chooses the
    /// calendar day (Berlin) by choosing the offset.
    pub value_date: OffsetDateTime,
    pub amount: Amount,
    pub payee_payer_name: String,
    pub status: TransactionStatus,
    pub check_account: ObjectRef<CheckAccountId>,
    pub options: NewTransactionOptions,
}

/// The optional fields of a [`NewTransaction`]. A field left `None` is not
/// sent. `payee_payer_acct_no` and `payee_payer_bank_code` were measured on
/// the update route (2026-11-22); the create route takes them over unmeasured.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct NewTransactionOptions {
    /// `entryDate`, sent in the same format as `value_date`. Measured
    /// on the sevDesk test tenant (2026-10-01): it is accepted independently
    /// of `valueDate`, reads back unchanged, and defaults to `valueDate` when
    /// omitted. The payment date of a booking (Invoice/Voucher `payDate`, the
    /// transaction log's `bookingDate`, the DATEV payment line) is the
    /// transaction's `valueDate`, not `entryDate`.
    pub entry_date: Option<OffsetDateTime>,
    /// Substring-matched by sevDesk's filter; compare exactly after reading.
    pub paymt_purpose: Option<String>,
    /// IBAN or account number of the other party. Measured: stored
    /// (2026-11-22, on update).
    pub payee_payer_acct_no: Option<String>,
    /// BIC or bank code of the other party. Measured: stored (2026-11-22, on
    /// update).
    pub payee_payer_bank_code: Option<String>,
    /// The "Geldbewegung" link: the transaction this one is rebooked from.
    pub source_transaction: Option<ObjectRef<TransactionId>>,
    pub target_transaction: Option<ObjectRef<TransactionId>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewTransactionWire {
    #[serde(with = "time::serde::rfc3339")]
    value_date: OffsetDateTime,
    #[serde(serialize_with = "serialize_amount")]
    amount: Amount,
    payee_payer_name: String,
    status: TransactionStatus,
    check_account: ObjectRef<CheckAccountId>,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    entry_date: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paymt_purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payee_payer_acct_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payee_payer_bank_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_transaction: Option<ObjectRef<TransactionId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_transaction: Option<ObjectRef<TransactionId>>,
}

impl From<NewTransaction> for NewTransactionWire {
    fn from(new: NewTransaction) -> Self {
        let NewTransaction {
            value_date,
            amount,
            payee_payer_name,
            status,
            check_account,
            options,
        } = new;
        let NewTransactionOptions {
            entry_date,
            paymt_purpose,
            payee_payer_acct_no,
            payee_payer_bank_code,
            source_transaction,
            target_transaction,
        } = options;
        Self {
            value_date,
            amount,
            payee_payer_name,
            status,
            check_account,
            entry_date,
            paymt_purpose,
            payee_payer_acct_no,
            payee_payer_bank_code,
            source_transaction,
            target_transaction,
        }
    }
}

impl Serialize for NewTransaction {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        NewTransactionWire::from(self.clone()).serialize(s)
    }
}

/// `PUT /CheckAccountTransaction/{id}`. The links are stored only on the
/// side they are set on, so a transfer sets both. All fields are optional and
/// the type has no `#[non_exhaustive]`, so `..Default::default()` keeps
/// working. Every field is measured: stored (2026-11-22), except the status
/// and the links (measured earlier).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TransactionStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_transaction: Option<ObjectRef<TransactionId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_transaction: Option<ObjectRef<TransactionId>>,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub value_date: Option<OffsetDateTime>,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub entry_date: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymt_purpose: Option<String>,
    /// Exact decimal string; carries the sign of the transaction.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_amount_opt"
    )]
    pub amount: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payee_payer_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payee_payer_acct_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payee_payer_bank_code: Option<String>,
    /// Moves the transaction to another check account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_account: Option<ObjectRef<CheckAccountId>>,
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
#[derive(Debug, Clone)]
pub struct CreateFileImportAccount {
    pub name: String,
    pub import_type: ImportType,
    pub options: FileImportAccountOptions,
}

/// The optional fields of a [`CreateFileImportAccount`]. A field left `None`
/// is not sent.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct FileImportAccountOptions {
    /// Without one, sevDesk assigns the next free booking account.
    pub accounting_number: Option<i64>,
    pub iban: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileImportAccountWire {
    name: String,
    import_type: ImportType,
    #[serde(skip_serializing_if = "Option::is_none")]
    accounting_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iban: Option<String>,
}

impl From<CreateFileImportAccount> for FileImportAccountWire {
    fn from(account: CreateFileImportAccount) -> Self {
        let CreateFileImportAccount {
            name,
            import_type,
            options,
        } = account;
        let FileImportAccountOptions {
            accounting_number,
            iban,
        } = options;
        Self {
            name,
            import_type,
            accounting_number,
            iban,
        }
    }
}

impl Serialize for CreateFileImportAccount {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        FileImportAccountWire::from(self.clone()).serialize(s)
    }
}
