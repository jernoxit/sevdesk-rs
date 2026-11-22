use crate::codec::{
    amount_opt, deserialize_flag_opt, deserialize_i64_opt, percent_bp_opt, string_opt,
};
use crate::ids::*;
use crate::status::*;
use crate::wire::Guard;
use crate::{Amount, CurrencyCode};
use serde::Deserialize;
use time::OffsetDateTime;

/// A CreditNote.
///
/// Every key a recording or the shape survey shows filled is a field. The
/// keys only the OpenAPI spec names say so in their doc; the keys no
/// measurement fills and the spec does not type (`accountEndDate`,
/// `accountIntervall`, `accountLastCreditNote`, `accountNextCreditNote`,
/// `additionalInformation`, `addressGender`, `addressName2`,
/// `addressParentName`, `addressParentName2`, `reminderCharge`,
/// `reminderDeadline`, `reminderDebit`, `reminderTotal`, `taxNumber`,
/// `timeToPay`, `vatNumber`) are not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CreditNote {
    pub id: CreditNoteId,
    /// May change at `sendBy` if the number was already taken (measured with
    /// automatic enshrinement on); read it after finalizing.
    #[serde(default)]
    pub credit_note_number: Option<String>,
    /// The customer. Only the id is read.
    #[serde(default)]
    pub contact: Option<ObjectRef<ContactId>>,
    /// When the credit note was created.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    /// Last change at sevDesk.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub credit_note_date: Option<OffsetDateTime>,
    /// The document title (`header`).
    #[serde(default)]
    pub header: Option<String>,
    /// The text above the positions; `null` except on 17 of 196 surveyed notes.
    #[serde(default)]
    pub head_text: Option<String>,
    /// The text below the positions; `null` in every measurement, so the form
    /// is not measured; type from the OpenAPI spec.
    #[serde(default)]
    pub foot_text: Option<String>,
    /// Days within which the discount (Skonto) applies; measured `"0"`, rarely `null`.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub discount_time: Option<i64>,
    /// Discount (Skonto), a pure percentage, in basis points; measured `"0"` only.
    #[serde(default, deserialize_with = "percent_bp_opt")]
    pub discount: Option<i64>,
    /// The document address sevDesk froze from these fields when the document
    /// was saved; read here, the draft payloads write them.
    #[serde(default, deserialize_with = "string_opt")]
    pub address_name: Option<String>,
    #[serde(default, deserialize_with = "string_opt")]
    pub address_street: Option<String>,
    /// A digit string, kept as text (a postcode is an identifier).
    #[serde(default, deserialize_with = "string_opt")]
    pub address_zip: Option<String>,
    #[serde(default, deserialize_with = "string_opt")]
    pub address_city: Option<String>,
    #[serde(default)]
    pub address_country: Option<ObjectRef<CountryId>>,
    /// When the credit note was paid in full; `None` while open.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub pay_date: Option<OffsetDateTime>,
    /// The sevDesk user who created the credit note.
    #[serde(default)]
    pub create_user: Option<ObjectRef<SevUserId>>,
    /// The sevDesk client (the account's own company) the credit note belongs to.
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    /// Start of the service period.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date: Option<OffsetDateTime>,
    #[serde(default)]
    pub status: Option<CreditNoteStatus>,
    /// Small-business settlement (Kleinunternehmer) switch; the wire has `"0"`/`"1"`.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub small_settlement: Option<bool>,
    /// The sevDesk user named as contact person on the document.
    #[serde(default)]
    pub contact_person: Option<ObjectRef<SevUserId>>,
    /// Tax rate in basis points (`1900` is 19 %); measured `0` only.
    #[serde(default, deserialize_with = "percent_bp_opt")]
    pub tax_rate: Option<i64>,
    #[serde(default)]
    pub tax_text: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub tax_type: Option<TaxType>,
    /// When the credit note was sent (`sendBy`); `None` for a draft.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub send_date: Option<OffsetDateTime>,
    /// Set only by `createFromInvoice` (measured); a credit note saved with
    /// `refSrcInvoice` has none.
    #[serde(default)]
    pub origin: Option<ObjectRef<InvoiceId>>,
    /// Measured as `"CN"` only; no value set is known, so it stays text.
    #[serde(default)]
    pub credit_note_type: Option<String>,
    /// The address as one text block (name, street, zip, city), with line breaks.
    #[serde(default)]
    pub address: Option<String>,
    pub currency: CurrencyCode,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discounts: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discounts_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_net_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_tax_accounting: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_gross_accounting: Option<Amount>,
    /// What has been paid so far. No credit-note measurement carries the key;
    /// the field predates the survey.
    #[serde(default, deserialize_with = "amount_opt")]
    pub paid_amount: Option<Amount>,
    #[serde(default)]
    pub customer_internal_note: Option<String>,
    /// Net display switch; the wire has `"0"`/`"1"` (`"0"` on gross notes).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub show_net: Option<bool>,
    /// When the credit note was enshrined; `None` for a draft.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub enshrined: Option<OffsetDateTime>,
    /// How the credit note was sent: `"0"` on drafts, `VPDF` once sent.
    #[serde(default)]
    pub send_type: Option<DocumentSendType>,
    #[serde(default)]
    pub booking_category: Option<CreditNoteBookingCategory>,
    /// End of the service period.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date_until: Option<OffsetDateTime>,
    /// The invoice this credit note refers to (`refSrcInvoice`).
    #[serde(default)]
    pub ref_src_invoice: Option<ObjectRef<InvoiceId>>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub is_transferred: Option<bool>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_gross_foreign_currency: Option<Amount>,
    /// The tax rule. Only the id is read.
    #[serde(default)]
    pub tax_rule: Option<ObjectRef<TaxRuleId>>,
    /// Measured on `saveCreditNote` answers (`"1"`).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub property_is_e_invoice: Option<bool>,
    #[serde(default)]
    pub payment_method: Option<ObjectRef<PaymentMethodId>>,
    /// Tax set. Form not measured; type from the OpenAPI spec.
    #[serde(default)]
    pub tax_set: Option<ObjectRef<TaxSetId>>,
    /// The accounting type; measured in the shape survey on 98 of 196 notes,
    /// in no recorded fixture. Only the id is read.
    #[serde(default)]
    pub accounting_type: Option<ObjectRef<AccountingTypeId>>,
}

impl Guard for CreditNote {
    const NAME: &'static str = "CreditNote";
    fn load_bearing_present(&self) -> bool {
        self.status.is_some() || self.sum_gross.is_some()
    }
}
