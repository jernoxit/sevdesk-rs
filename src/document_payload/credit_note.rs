//! The credit note payload.
//!
//! Measured on the test account (2026-09-29): a credit note needs a number
//! from its sequence ([`crate::SevdeskClient::next_credit_note_number`]) and a
//! `bookingCategory`. `UNDERACHIEVEMENT` demands the invoice it refers to
//! (`refSrcInvoice`) and that every position carries that invoice's tax rate:
//! without the reference, or with another rate, sevDesk answers 456
//! ("Different taxRate than origin document"). `PROVISION` takes tax rule 9
//! (rule 1 answers "Invalid tax rule"). `creditNoteDate` goes out as a
//! `dd.mm.yyyy` string, the service period as Unix seconds.

use super::DocumentPositionDraft;
use super::position::{PositionKind, PositionWire};
use crate::CurrencyCode;
use crate::ids::*;
use crate::payload::{ser_german_date, ser_german_date_opt, ser_unix, ser_unix_opt};
use crate::status::*;
use serde::{Serialize, Serializer};
use time::{Date, OffsetDateTime};

/// The document part of `saveCreditNote`. The named fields are what a request
/// cannot do without (and what sevDesk would otherwise default silently);
/// everything else is in [`Self::options`]. Every business value is the
/// caller's; nothing is defaulted.
#[derive(Debug, Clone)]
pub struct CreditNoteDraft {
    /// From [`crate::SevdeskClient::next_credit_note_number`]; without one,
    /// `sendBy` fails. A number already taken is not refused but replaced at
    /// `sendBy`.
    pub credit_note_number: String,
    pub contact: ObjectRef<ContactId>,
    pub contact_person: ObjectRef<SevUserId>,
    pub credit_note_date: Date,
    /// Start of the service period, as for [`crate::InvoiceDraft`].
    pub delivery_date: OffsetDateTime,
    pub header: String,
    pub tax_rule: ObjectRef<TaxRuleId>,
    pub tax_text: String,
    pub booking_category: CreditNoteCategory,
    pub currency: CurrencyCode,
    /// `false`: the positions carry gross prices.
    pub show_net: bool,
    /// A free reference; `GET /CreditNote` filters on it EXACTLY.
    pub customer_internal_note: String,
    pub options: CreditNoteOptions,
}

/// The optional fields of a [`CreditNoteDraft`].
///
/// Every field is optional; a field left `None` is not sent. Set them by
/// assignment: `draft.options.foot_text = Some("…".into())`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct CreditNoteOptions {
    /// End of the service period.
    pub delivery_date_until: Option<OffsetDateTime>,
    pub head_text: Option<String>,
    pub foot_text: Option<String>,
    /// The invoice this credit note refers to.
    pub ref_src_invoice: Option<ObjectRef<InvoiceId>>,
    pub address_name: Option<String>,
    pub address_street: Option<String>,
    pub address_zip: Option<String>,
    pub address_city: Option<String>,
    pub address_country: Option<ObjectRef<CountryId>>,
    /// Required for an e-invoice credit note: without it rendering the draft
    /// answers 830 (HTTP 400, missing `document_paymentMethod`; measured
    /// 2026-10-01).
    pub payment_method: Option<ObjectRef<PaymentMethodId>>,
    /// Sent as `propertyIsEInvoice: true` when set, left out otherwise.
    /// sevDesk reads it back as `null`, so the credit note does not report it.
    pub is_e_invoice: bool,
    /// The whole address as one text, lines separated by `\n`; measured:
    /// stored (2026-11-22). Independent of the single `address_*` fields.
    pub address: Option<String>,
    /// Small-business scheme (`smallSettlement`); measured: stored
    /// (2026-11-22).
    pub small_settlement: Option<bool>,
    /// The tax set of the "Update 1.0" tax model; measured: stored
    /// (2026-11-22). `tax_rule` stays the model this crate uses.
    pub tax_set: Option<ObjectRef<TaxSetId>>,
    /// Only records a date on the draft; it does not send anything (measured
    /// 2026-11-22: stored, status stays 100). Sent as `dd.mm.yyyy`.
    #[deprecated(
        note = "DO NOT USE: only records a send date on the draft without sending anything; sendBy sets it when it finalizes (measured 2026-11-22)"
    )]
    pub send_date: Option<Date>,
    /// Only records how the draft was "sent"; it does not finalize it
    /// (measured 2026-11-22: stored, status stays 100).
    #[deprecated(
        note = "DO NOT USE: only records a send type on the draft without sending anything; sendBy sets it when it finalizes (measured 2026-11-22)"
    )]
    pub send_type: Option<SendType>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreditNoteWire {
    object_name: &'static str,
    map_all: bool,
    credit_note_number: String,
    contact: ObjectRef<ContactId>,
    contact_person: ObjectRef<SevUserId>,
    #[serde(serialize_with = "ser_german_date")]
    credit_note_date: Date,
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
    status: CreditNoteStatus,
    tax_rule: ObjectRef<TaxRuleId>,
    tax_rate: i64,
    tax_text: String,
    credit_note_type: &'static str,
    booking_category: CreditNoteCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_src_invoice: Option<ObjectRef<InvoiceId>>,
    currency: CurrencyCode,
    show_net: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tax_set: Option<ObjectRef<TaxSetId>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_german_date_opt"
    )]
    send_date: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_type: Option<SendType>,
}

impl From<CreditNoteDraft> for CreditNoteWire {
    // The crate itself is the one place that reads the deprecated
    // `send_date` and `send_type`: the wire form must carry them when set.
    #[allow(
        deprecated,
        reason = "encodes the deprecated send_date and send_type options"
    )]
    fn from(draft: CreditNoteDraft) -> Self {
        let CreditNoteDraft {
            credit_note_number,
            contact,
            contact_person,
            credit_note_date,
            delivery_date,
            header,
            tax_rule,
            tax_text,
            booking_category,
            currency,
            show_net,
            customer_internal_note,
            options,
        } = draft;
        let CreditNoteOptions {
            delivery_date_until,
            head_text,
            foot_text,
            ref_src_invoice,
            address_name,
            address_street,
            address_zip,
            address_city,
            address_country,
            payment_method,
            is_e_invoice,
            address,
            small_settlement,
            tax_set,
            send_date,
            send_type,
        } = options;
        Self {
            object_name: "CreditNote",
            map_all: true,
            credit_note_number,
            contact,
            contact_person,
            credit_note_date,
            delivery_date,
            delivery_date_until,
            header,
            head_text,
            foot_text,
            discount: 0,
            status: CreditNoteStatus::Draft,
            tax_rule,
            tax_rate: 0,
            tax_text,
            credit_note_type: "CN",
            booking_category,
            ref_src_invoice,
            currency,
            show_net,
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
            tax_set,
            send_date,
            send_type,
        }
    }
}

/// `POST /CreditNote/Factory/saveCreditNote`.
#[derive(Debug, Clone)]
pub struct SaveCreditNote {
    pub credit_note: CreditNoteDraft,
    pub positions: Vec<DocumentPositionDraft>,
}

impl SaveCreditNote {
    pub fn new(credit_note: CreditNoteDraft, positions: Vec<DocumentPositionDraft>) -> Self {
        Self {
            credit_note,
            positions,
        }
    }
}

#[derive(Debug, Serialize)]
struct SaveCreditNoteWire {
    #[serde(rename = "creditNote")]
    credit_note: CreditNoteWire,
    #[serde(rename = "creditNotePosSave")]
    positions: Vec<PositionWire>,
    #[serde(rename = "creditNotePosDelete")]
    positions_delete: Option<()>,
}

impl From<SaveCreditNote> for SaveCreditNoteWire {
    fn from(save: SaveCreditNote) -> Self {
        let SaveCreditNote {
            credit_note,
            positions,
        } = save;
        Self {
            credit_note: credit_note.into(),
            positions: positions
                .into_iter()
                .map(|p| PositionWire::new(PositionKind::CreditNote, p))
                .collect(),
            positions_delete: None,
        }
    }
}

impl Serialize for SaveCreditNote {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SaveCreditNoteWire::from(self.clone()).serialize(s)
    }
}
