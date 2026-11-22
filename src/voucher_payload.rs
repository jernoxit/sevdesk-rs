//! The `saveVoucher` payload: the draft, its positions and the call itself.
//!
//! Confirmed against the test account (`tests/live_write/`): `dd.mm.yyyy`
//! voucher dates come back as Berlin midnight (`2026-09-29T00:00:00+02:00`),
//! and a voucher is only updatable as a draft (see
//! [`crate::SevdeskClient::reset_voucher_to_draft`]).

use crate::codec::serialize_amount;
use crate::ids::*;
use crate::payload::{ser_german_date, ser_german_date_opt, ser_percent_bp};
use crate::status::*;
use crate::{Amount, CurrencyCode};
use serde::{Serialize, Serializer};
use time::Date;

/// The voucher part of a `saveVoucher` call. The named fields are what a
/// request cannot do without; everything else is in [`Self::options`].
#[derive(Debug, Clone)]
pub struct VoucherDraft {
    pub voucher_date: Date,
    /// Free text supplier; left out when empty (a `supplier` contact in the
    /// options names it).
    pub supplier_name: String,
    pub description: String,
    pub status: VoucherStatus,
    pub credit_debit: CreditDebit,
    pub voucher_type: VoucherType,
    pub tax_rule: ObjectRef<TaxRuleId>,
    pub currency: CurrencyCode,
    pub options: VoucherOptions,
}

/// The optional fields of a [`VoucherDraft`].
///
/// Every field is optional; a field left `None` is not sent. Set them by
/// assignment: `draft.options.pay_date = Some(date)`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct VoucherOptions {
    /// `Some` updates that voucher; `None` creates one.
    pub id: Option<VoucherId>,
    pub delivery_date: Option<Date>,
    pub delivery_date_until: Option<Date>,
    /// The supplier as a contact of the account.
    pub supplier: Option<ObjectRef<ContactId>>,
    /// Payment deadline of the voucher; measured: stored (2026-11-22).
    pub payment_deadline: Option<Date>,
    /// An uploaded document attached to the voucher; measured: stored
    /// (2026-11-22).
    pub document: Option<ObjectRef<DocumentId>>,
    /// Only records a date on the draft; it does not book a payment (measured:
    /// stored, 2026-11-22). Payments come from `book_voucher_amount`.
    pub pay_date: Option<Date>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoucherWire {
    object_name: &'static str,
    map_all: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<VoucherId>,
    #[serde(serialize_with = "ser_german_date")]
    voucher_date: Date,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    delivery_date: Option<Date>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    delivery_date_until: Option<Date>,
    #[serde(skip_serializing_if = "String::is_empty")]
    supplier_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    supplier: Option<ObjectRef<ContactId>>,
    description: String,
    status: VoucherStatus,
    credit_debit: CreditDebit,
    voucher_type: VoucherType,
    tax_rule: ObjectRef<TaxRuleId>,
    currency: CurrencyCode,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    payment_deadline: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<ObjectRef<DocumentId>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    pay_date: Option<Date>,
}

impl From<VoucherDraft> for VoucherWire {
    fn from(draft: VoucherDraft) -> Self {
        let VoucherDraft {
            voucher_date,
            supplier_name,
            description,
            status,
            credit_debit,
            voucher_type,
            tax_rule,
            currency,
            options,
        } = draft;
        let VoucherOptions {
            id,
            delivery_date,
            delivery_date_until,
            supplier,
            payment_deadline,
            document,
            pay_date,
        } = options;
        Self {
            object_name: "Voucher",
            map_all: true,
            id,
            voucher_date,
            delivery_date,
            delivery_date_until,
            supplier_name,
            supplier,
            description,
            status,
            credit_debit,
            voucher_type,
            tax_rule,
            currency,
            payment_deadline,
            document,
            pay_date,
        }
    }
}

impl VoucherDraft {
    /// A new draft expense voucher (`status` 50, `creditDebit` C, `voucherType`
    /// VOU) with no options; adjust the public fields for anything else.
    /// Measured with EUR only; per spec a currency other than the account's
    /// also needs an exchange rate, which this draft does not send.
    pub fn new(
        voucher_date: Date,
        supplier_name: impl Into<String>,
        description: impl Into<String>,
        tax_rule: TaxRuleId,
        currency: CurrencyCode,
    ) -> Self {
        Self {
            voucher_date,
            supplier_name: supplier_name.into(),
            description: description.into(),
            status: VoucherStatus::Draft,
            credit_debit: CreditDebit::Credit,
            voucher_type: VoucherType::Normal,
            tax_rule: ObjectRef::new(tax_rule),
            currency,
            options: VoucherOptions::default(),
        }
    }
}

/// One position of a `saveVoucher` call. The named fields are what a request
/// cannot do without; everything else is in [`Self::options`].
#[derive(Debug, Clone)]
pub struct VoucherPositionDraft {
    pub account_datev: ObjectRef<AccountDatevId>,
    /// Basis points: `1900` for 19 %.
    pub tax_rate_bp: i64,
    /// The net amount (sent with `net: true`, unless
    /// [`VoucherPositionOptions::sum_gross`] is set).
    pub sum_net: Amount,
    pub comment: String,
    pub options: VoucherPositionOptions,
}

/// The optional fields of a [`VoucherPositionDraft`].
///
/// Every field is optional; a field left `None` is not sent. Set them by
/// assignment: `position.options.is_asset = Some(true)`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct VoucherPositionOptions {
    /// Present when the position already exists. **On an update every
    /// position must be sent with its id**: sevDesk computes the voucher
    /// total from the positions of THIS call only.
    pub id: Option<VoucherPositionId>,
    /// The booking account of the position (`accountingType`); measured:
    /// stored (2026-11-22).
    pub accounting_type: Option<ObjectRef<AccountingTypeId>>,
    /// Whether the position is an asset to depreciate; measured: stored
    /// (2026-11-22).
    pub is_asset: Option<bool>,
    /// The gross sum. Setting it sends the position with `net: false`
    /// (measured 2026-11-22: with `net: true` sevDesk ignores `sumGross`; with
    /// `net: false` it stores it), and sevDesk then recomputes `sumNet` from
    /// it, ignoring the always-sent [`VoucherPositionDraft::sum_net`]
    /// (measured 2026-11-22 at 0 % only).
    pub sum_gross: Option<Amount>,
}

fn ser_amount_opt<S: Serializer>(amount: &Option<Amount>, s: S) -> Result<S::Ok, S::Error> {
    match amount {
        Some(amount) => serialize_amount(amount, s),
        None => s.serialize_none(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoucherPositionWire {
    object_name: &'static str,
    map_all: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<VoucherPositionId>,
    account_datev: ObjectRef<AccountDatevId>,
    #[serde(rename = "taxRate", serialize_with = "ser_percent_bp")]
    tax_rate_bp: i64,
    /// `true` unless a gross sum is sent.
    net: bool,
    #[serde(serialize_with = "serialize_amount")]
    sum_net: Amount,
    comment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    accounting_type: Option<ObjectRef<AccountingTypeId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_asset: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_amount_opt"
    )]
    sum_gross: Option<Amount>,
}

impl From<VoucherPositionDraft> for VoucherPositionWire {
    fn from(draft: VoucherPositionDraft) -> Self {
        let VoucherPositionDraft {
            account_datev,
            tax_rate_bp,
            sum_net,
            comment,
            options,
        } = draft;
        let VoucherPositionOptions {
            id,
            accounting_type,
            is_asset,
            sum_gross,
        } = options;
        Self {
            object_name: "VoucherPos",
            map_all: true,
            id,
            account_datev,
            tax_rate_bp,
            net: sum_gross.is_none(),
            sum_net,
            comment,
            accounting_type,
            is_asset,
            sum_gross,
        }
    }
}

impl VoucherPositionDraft {
    pub fn new(
        account_datev: AccountDatevId,
        tax_rate_bp: i64,
        sum_net: Amount,
        comment: impl Into<String>,
    ) -> Self {
        Self {
            account_datev: ObjectRef::new(account_datev),
            tax_rate_bp,
            sum_net,
            comment: comment.into(),
            options: VoucherPositionOptions::default(),
        }
    }

    /// The same position as it already exists at sevDesk.
    pub fn with_id(mut self, id: VoucherPositionId) -> Self {
        self.options.id = Some(id);
        self
    }
}

/// `POST /Voucher/Factory/saveVoucher`: create or update a draft.
#[derive(Debug, Clone)]
pub struct SaveVoucher {
    pub voucher: VoucherDraft,
    pub positions: Vec<VoucherPositionDraft>,
    /// The `filename` returned by `uploadTempFile`; attaches the PDF.
    pub filename: Option<String>,
}

#[derive(Debug, Serialize)]
struct SaveVoucherWire {
    voucher: VoucherWire,
    #[serde(rename = "voucherPosSave")]
    positions: Vec<VoucherPositionWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

impl From<SaveVoucher> for SaveVoucherWire {
    fn from(save: SaveVoucher) -> Self {
        let SaveVoucher {
            voucher,
            positions,
            filename,
        } = save;
        Self {
            voucher: voucher.into(),
            positions: positions.into_iter().map(Into::into).collect(),
            filename,
        }
    }
}

impl Serialize for SaveVoucher {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SaveVoucherWire::from(self.clone()).serialize(s)
    }
}
