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
use crate::payload::{ser_german_date, ser_unix, ser_unix_opt};
use crate::status::*;
use serde::{Serialize, Serializer};
use time::{Date, OffsetDateTime};

/// The document part of `saveCreditNote`. Every business value is the
/// caller's; nothing is defaulted.
#[derive(Debug, Clone)]
pub struct CreditNoteDraft {
    /// From [`crate::SevdeskClient::next_credit_note_number`]; without one,
    /// `sendBy` fails.
    pub credit_note_number: String,
    pub contact: ObjectRef<ContactId>,
    pub contact_person: ObjectRef<SevUserId>,
    pub credit_note_date: Date,
    /// Start of the service period, as for [`crate::InvoiceDraft`].
    pub delivery_date: OffsetDateTime,
    /// End of the service period; left out when `None`.
    pub delivery_date_until: Option<OffsetDateTime>,
    pub header: String,
    pub head_text: Option<String>,
    pub foot_text: Option<String>,
    pub tax_rule: ObjectRef<TaxRuleId>,
    pub tax_text: String,
    pub booking_category: CreditNoteCategory,
    /// The invoice this credit note refers to; left out when `None`.
    pub ref_src_invoice: Option<ObjectRef<InvoiceId>>,
    pub currency: CurrencyCode,
    /// `false`: the positions carry gross prices.
    pub show_net: bool,
    /// A free reference; `GET /CreditNote` filters on it EXACTLY.
    pub customer_internal_note: String,
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
}

impl From<CreditNoteDraft> for CreditNoteWire {
    fn from(draft: CreditNoteDraft) -> Self {
        let CreditNoteDraft {
            credit_note_number,
            contact,
            contact_person,
            credit_note_date,
            delivery_date,
            delivery_date_until,
            header,
            head_text,
            foot_text,
            tax_rule,
            tax_text,
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
        } = draft;
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
