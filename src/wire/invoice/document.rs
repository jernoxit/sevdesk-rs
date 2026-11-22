use crate::codec::{
    amount_opt, deserialize_flag_opt, deserialize_i64_opt, percent_bp_opt, string_opt,
};
use crate::ids::*;
use crate::status::*;
use crate::wire::{AdditionalInformation, Guard};
use crate::{Amount, CurrencyCode};
use serde::Deserialize;
use time::OffsetDateTime;

/// An Invoice; a cancellation invoice (`SR`) points to its original via `origin`.
///
/// Every key a recording shows filled is a field. The keys only the OpenAPI
/// spec names say so in their doc; the keys no recording fills and the spec
/// does not type (`accountEndDate`, `accountLastInvoice`, `accountStartDate`,
/// `addressGender`, `addressName2`, `addressParentName`, `addressParentName2`,
/// `datevConnectOnline`, `propertyIsBookkeepingV1`, `sepaCreditorId`,
/// `sepaMandateReference`, `sepaPayerIban`) are not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id: InvoiceId,
    /// Free-form object, `null` on most invoices; only `creditVoucher` is measured.
    #[serde(default)]
    pub additional_information: Option<AdditionalInformation>,
    #[serde(default)]
    pub invoice_number: Option<String>,
    /// The customer. A full `Contact` object comes with `cancelInvoice`; only the id is read.
    #[serde(default)]
    pub contact: Option<ObjectRef<ContactId>>,
    /// When the invoice was created.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    /// Last change at sevDesk; what `updateAfter` filters on.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The sevDesk client (the account's own company) the invoice belongs to.
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub invoice_date: Option<OffsetDateTime>,
    /// The document title (`header`).
    #[serde(default)]
    pub header: Option<String>,
    /// The text above the positions.
    #[serde(default)]
    pub head_text: Option<String>,
    /// The text below the positions (HTML, measured).
    #[serde(default)]
    pub foot_text: Option<String>,
    /// Days until the invoice is due.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub time_to_pay: Option<i64>,
    /// Days within which the discount (Skonto) applies; measured `0` once, else `null`.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub discount_time: Option<i64>,
    /// Discount (Skonto), a pure percentage per the spec ("define the
    /// percentage here"), in basis points; measured `0` only.
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
    /// When the invoice was paid in full; `None` while open.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub pay_date: Option<OffsetDateTime>,
    /// The sevDesk user who created the invoice.
    #[serde(default)]
    pub create_user: Option<ObjectRef<SevUserId>>,
    /// Start of the service period.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date: Option<OffsetDateTime>,
    #[serde(default)]
    pub status: Option<InvoiceStatus>,
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
    /// Dunning level. Form not measured; type from the OpenAPI spec (string).
    #[serde(default)]
    pub dunning_level: Option<String>,
    /// Form not measured (`null` in every recording); type from the OpenAPI spec.
    #[serde(default)]
    pub tax_type: Option<TaxType>,
    /// When the invoice was sent (`sendBy`); `None` for a draft.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub send_date: Option<OffsetDateTime>,
    #[serde(default)]
    pub invoice_type: Option<InvoiceType>,
    /// Recurrence interval. Form not measured; type from the OpenAPI spec. The
    /// key keeps its wire spelling with the double `l`.
    #[serde(default)]
    pub account_intervall: Option<String>,
    /// When a recurring invoice is generated next. Form not measured; type
    /// from the OpenAPI spec (string; the request spec says integer).
    #[serde(default)]
    pub account_next_invoice: Option<String>,
    /// Reminder total. Form not measured; type from the OpenAPI spec.
    #[serde(default, deserialize_with = "amount_opt")]
    pub reminder_total: Option<Amount>,
    /// Reminder debit. Form not measured; type from the OpenAPI spec.
    #[serde(default, deserialize_with = "amount_opt")]
    pub reminder_debit: Option<Amount>,
    /// Reminder deadline. Form not measured; type from the OpenAPI spec.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub reminder_deadline: Option<OffsetDateTime>,
    /// Reminder charge; measured `"0"` only.
    #[serde(default, deserialize_with = "amount_opt")]
    pub reminder_charge: Option<Amount>,
    /// The address as one text block (name, street, zip, city), with line breaks.
    #[serde(default, deserialize_with = "string_opt")]
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
    /// What has been paid so far; a JSON number on the wire, unlike the sums.
    #[serde(default, deserialize_with = "amount_opt")]
    pub paid_amount: Option<Amount>,
    #[serde(default)]
    pub customer_internal_note: Option<String>,
    /// E-invoice reference; measured `"CUSTOM"` once, else `null`.
    #[serde(default)]
    pub einvoice_reference: Option<String>,
    /// Net display switch; the wire has `"0"`/`"1"`.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub show_net: Option<bool>,
    /// When the invoice was enshrined (finalized); `None` for a draft.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub enshrined: Option<OffsetDateTime>,
    /// How the invoice was sent: `"0"` on drafts, `VPDF` once sent.
    #[serde(default)]
    pub send_type: Option<DocumentSendType>,
    /// End of the service period.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub delivery_date_until: Option<OffsetDateTime>,
    /// Payment-received notification date. Form not measured (`null` in every
    /// recording); the spec says a plain string, read as text.
    #[serde(default)]
    pub send_payment_received_notification_date: Option<String>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_net: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_gross: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_net_foreign_currency: Option<Amount>,
    #[serde(default, deserialize_with = "amount_opt")]
    pub sum_discount_gross_foreign_currency: Option<Amount>,
    /// The tax rule. A full `TaxRule` object comes with `cancelInvoice`; only the id is read.
    #[serde(default)]
    pub tax_rule: Option<ObjectRef<TaxRuleId>>,
    #[serde(default)]
    pub payment_method: Option<ObjectRef<PaymentMethodId>>,
    /// Cost centre. Form not measured; type from the OpenAPI spec.
    #[serde(default)]
    pub cost_centre: Option<ObjectRef<CostCentreId>>,
    /// Tax set. Form not measured; type from the OpenAPI spec.
    #[serde(default)]
    pub tax_set: Option<ObjectRef<TaxSetId>>,
    /// The original invoice of a cancellation invoice (measured as `Invoice`;
    /// the spec's example says `Order`).
    #[serde(default)]
    pub origin: Option<ObjectRef<InvoiceId>>,
    /// Measured on `cancelInvoice` answers only (`"1"`).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub property_use_new_doc_server: Option<bool>,
    /// Measured on `saveInvoice` and `cancelInvoice` answers (`"1"` or `null`).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub property_is_e_invoice: Option<bool>,
    /// Measured on `cancelInvoice` answers only (`"1"`).
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub property_use_new_calculation: Option<bool>,
}

impl Invoice {
    /// This is a cancellation invoice (`SR`) whose `origin` is `original`.
    pub fn cancels(&self, original: InvoiceId) -> bool {
        self.invoice_type == Some(InvoiceType::Cancellation)
            && self.origin.is_some_and(|origin| origin.id == original)
    }
}

impl Guard for Invoice {
    const NAME: &'static str = "Invoice";
    fn load_bearing_present(&self) -> bool {
        self.status.is_some() || self.sum_gross.is_some()
    }
}
