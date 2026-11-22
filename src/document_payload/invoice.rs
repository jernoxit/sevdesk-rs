//! The invoice payload.
//!
//! Measured on the test account (2026-09-29): `saveInvoice` makes a DRAFT
//! (`status` 100, no number); `sendBy` with [`crate::SendType::Vpdf`] finalizes
//! it (status 200) and only then assigns the number. `invoiceDate` goes out as
//! a `dd.mm.yyyy` string, the service period as Unix seconds. sevDesk does not
//! copy the contact's address onto the document: send the address fields, or
//! they stay empty.

use super::DocumentPositionDraft;
use super::position::{PositionKind, PositionWire};
use crate::CurrencyCode;
use crate::ids::*;
use crate::payload::{ser_german_date, ser_german_date_opt, ser_unix, ser_unix_opt};
use crate::status::*;
use serde::{Serialize, Serializer};
use time::{Date, OffsetDateTime};

/// The document part of `saveInvoice`. The named fields are what a request
/// cannot do without (and what sevDesk would otherwise default silently);
/// everything else is in [`Self::options`]. Every business value is the
/// caller's; nothing is defaulted.
#[derive(Debug, Clone)]
pub struct InvoiceDraft {
    pub contact: ObjectRef<ContactId>,
    pub contact_person: ObjectRef<SevUserId>,
    pub invoice_date: Date,
    /// Start of the service period. The caller computes the instant (Berlin
    /// midnight, for instance); it goes out as Unix seconds.
    pub delivery_date: OffsetDateTime,
    pub header: String,
    pub tax_rule: ObjectRef<TaxRuleId>,
    pub tax_text: String,
    pub currency: CurrencyCode,
    /// `false`: the positions carry gross prices.
    pub show_net: bool,
    /// Days until the invoice is due.
    pub time_to_pay: i64,
    /// A free reference; `GET /Invoice` filters on it EXACTLY.
    pub customer_internal_note: String,
    pub options: InvoiceOptions,
}

/// The optional fields of an [`InvoiceDraft`].
///
/// Every field is optional; a field left `None` is not sent. Set them by
/// assignment: `draft.options.foot_text = Some("…".into())`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct InvoiceOptions {
    /// End of the service period.
    pub delivery_date_until: Option<OffsetDateTime>,
    pub head_text: Option<String>,
    pub foot_text: Option<String>,
    pub address_name: Option<String>,
    pub address_street: Option<String>,
    pub address_zip: Option<String>,
    pub address_city: Option<String>,
    pub address_country: Option<ObjectRef<CountryId>>,
    pub payment_method: Option<ObjectRef<PaymentMethodId>>,
    /// Sent as `propertyIsEInvoice: true` when set, left out otherwise.
    /// sevDesk reads it back as `null`, so the invoice does not report it.
    pub is_e_invoice: bool,
    /// The whole address as one text, lines separated by `\n`; measured:
    /// stored (2026-11-22). Independent of the single `address_*` fields.
    pub address: Option<String>,
    /// Small-business scheme (`smallSettlement`); measured: stored
    /// (2026-11-22).
    pub small_settlement: Option<bool>,
    /// Only records a date on the draft; it does not send anything (measured
    /// 2026-11-22: stored, status stays 100). Sent as `dd.mm.yyyy`.
    #[deprecated(
        note = "DO NOT USE: only records a send date on the draft without sending anything; sendBy sets it when it finalizes (measured 2026-11-22)"
    )]
    pub send_date: Option<Date>,
    /// Only records a date on the draft; it does not book a payment (measured
    /// 2026-11-22: stored, status stays 100). Sent as `dd.mm.yyyy`.
    pub pay_date: Option<Date>,
    /// Only records how the draft was "sent"; it does not finalize it
    /// (measured 2026-11-22: stored, status stays 100, no number assigned).
    #[deprecated(
        note = "DO NOT USE: only records a send type on the draft without sending anything; sendBy sets it when it finalizes (measured 2026-11-22)"
    )]
    pub send_type: Option<SendType>,
    /// DO NOT USE: numbers come from `sendBy`. Measured 2026-11-22: a custom
    /// number set on a draft REPLACES a number the draft already had (that
    /// number is then a gap in the sequence; it only had one because the draft
    /// was re-saved through the save factory with its id), and a custom number
    /// is kept at `sendBy`, bypassing the sequence.
    #[deprecated(
        note = "DO NOT USE: a custom number replaces a number the draft already had (a gap in the sequence) and is kept at sendBy, bypassing the sequence (measured 2026-11-22); numbers come from sendBy"
    )]
    pub invoice_number: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceWire {
    object_name: &'static str,
    map_all: bool,
    contact: ObjectRef<ContactId>,
    contact_person: ObjectRef<SevUserId>,
    #[serde(serialize_with = "ser_german_date")]
    invoice_date: Date,
    #[serde(serialize_with = "ser_unix")]
    delivery_date: OffsetDateTime,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_unix_opt"
    )]
    delivery_date_until: Option<OffsetDateTime>,
    header: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    head_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foot_text: Option<String>,
    discount: i64,
    status: InvoiceStatus,
    tax_rule: ObjectRef<TaxRuleId>,
    tax_rate: i64,
    tax_text: String,
    invoice_type: InvoiceType,
    currency: CurrencyCode,
    show_net: bool,
    time_to_pay: i64,
    customer_internal_note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_zip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_country: Option<ObjectRef<CountryId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_method: Option<ObjectRef<PaymentMethodId>>,
    #[serde(
        rename = "propertyIsEInvoice",
        skip_serializing_if = "std::ops::Not::not"
    )]
    is_e_invoice: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    small_settlement: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    send_date: Option<Date>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    pay_date: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_type: Option<SendType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invoice_number: Option<String>,
}

impl From<InvoiceDraft> for InvoiceWire {
    // The crate itself is the one place that reads the deprecated
    // `invoice_number`, `send_date` and `send_type`: the wire form must carry it when a caller sets it.
    #[allow(
        deprecated,
        reason = "encodes the deprecated invoice_number, send_date and send_type options"
    )]
    fn from(draft: InvoiceDraft) -> Self {
        let InvoiceDraft {
            contact,
            contact_person,
            invoice_date,
            delivery_date,
            header,
            tax_rule,
            tax_text,
            currency,
            show_net,
            time_to_pay,
            customer_internal_note,
            options,
        } = draft;
        let InvoiceOptions {
            delivery_date_until,
            head_text,
            foot_text,
            address_name,
            address_street,
            address_zip,
            address_city,
            address_country,
            payment_method,
            is_e_invoice,
            address,
            small_settlement,
            send_date,
            pay_date,
            send_type,
            invoice_number,
        } = options;
        Self {
            object_name: "Invoice",
            map_all: true,
            contact,
            contact_person,
            invoice_date,
            delivery_date,
            delivery_date_until,
            header,
            head_text,
            foot_text,
            discount: 0,
            status: InvoiceStatus::Draft,
            tax_rule,
            tax_rate: 0,
            tax_text,
            invoice_type: InvoiceType::Invoice,
            currency,
            show_net,
            time_to_pay,
            customer_internal_note,
            address_name,
            address_street,
            address_zip,
            address_city,
            address_country,
            payment_method,
            is_e_invoice,
            address,
            small_settlement,
            send_date,
            pay_date,
            send_type,
            invoice_number,
        }
    }
}

/// `POST /Invoice/Factory/saveInvoice`. Creates a draft; measured 2026-11-22:
/// saving an existing draft again through this factory assigns it a number
/// (see [`crate::SevdeskClient::save_invoice`]).
#[derive(Debug, Clone)]
pub struct SaveInvoice {
    pub invoice: InvoiceDraft,
    pub positions: Vec<DocumentPositionDraft>,
}

impl SaveInvoice {
    pub fn new(invoice: InvoiceDraft, positions: Vec<DocumentPositionDraft>) -> Self {
        Self { invoice, positions }
    }
}

#[derive(Debug, Serialize)]
struct SaveInvoiceWire {
    invoice: InvoiceWire,
    #[serde(rename = "invoicePosSave")]
    positions: Vec<PositionWire>,
    #[serde(rename = "invoicePosDelete")]
    positions_delete: Option<()>,
}

impl From<SaveInvoice> for SaveInvoiceWire {
    fn from(save: SaveInvoice) -> Self {
        let SaveInvoice { invoice, positions } = save;
        Self {
            invoice: invoice.into(),
            positions: positions
                .into_iter()
                .map(|p| PositionWire::new(PositionKind::Invoice, p))
                .collect(),
            positions_delete: None,
        }
    }
}

impl Serialize for SaveInvoice {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SaveInvoiceWire::from(self.clone()).serialize(s)
    }
}
