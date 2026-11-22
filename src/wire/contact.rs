use super::Guard;
use crate::codec::{date_opt, deserialize_flag_opt, string_opt};
use crate::ids::*;
use crate::status::{CommunicationWayType, ContactStatus};
use serde::Deserialize;
use time::{Date, OffsetDateTime};

/// A Contact (customer). An organisation has `name`, a person `surename` and
/// `familyname` (measured: `name` is `null` for a person).
///
/// Every key a recording or the shape survey shows filled is a field, and so
/// is every key the OpenAPI spec names. The keys only the spec names say so in
/// their doc; `mandateReference` and `taxType` are `null` in every measurement
/// and not in the spec (form unmeasured), so they are not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: ContactId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default)]
    pub name: Option<String>,
    /// The first name (sevDesk's spelling).
    #[serde(default)]
    pub surename: Option<String>,
    #[serde(default)]
    pub familyname: Option<String>,
    /// A second name of a person. Form not measured (`null` in every
    /// measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub name2: Option<String>,
    /// A non-academic title. Form not measured (`null` in every measurement);
    /// type from the OpenAPI spec.
    #[serde(default)]
    pub titel: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub academic_title: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI
    /// spec, which says "m, w, or your own definition": free text, so no enum.
    #[serde(default)]
    pub gender: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI
    /// spec (`date`).
    #[serde(default, deserialize_with = "date_opt")]
    pub birthday: Option<Date>,
    /// The contact's status; measured `1000` only.
    #[serde(default)]
    pub status: Option<ContactStatus>,
    /// A string on the wire, measured as digits (`"1000"`) and as text.
    #[serde(default)]
    pub customer_number: Option<String>,
    /// Measured as a reference only (`{"id": "3", "objectName": "Category"}`),
    /// never as a full object.
    #[serde(default)]
    pub category: Option<ObjectRef<ContactCategoryId>>,
    /// The organisation a person belongs to. In no recording or survey object;
    /// the form is from the OpenAPI spec.
    #[serde(default)]
    pub parent: Option<ObjectRef<ContactId>>,
    #[serde(default)]
    pub description: Option<String>,
    /// Measured as text, `null` and `""` (read as `None`).
    #[serde(default, deserialize_with = "string_opt")]
    pub vat_number: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub tax_number: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI
    /// spec (only for Greek customers).
    #[serde(default)]
    pub tax_office: Option<String>,
    /// The IBAN. Form not measured (`null` in every measurement); type from
    /// the OpenAPI spec.
    #[serde(default)]
    pub bank_account: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub bank_number: Option<String>,
    /// Days in which paying earns a cashback. Form not measured (`null` in
    /// every measurement); type from the OpenAPI spec (a string).
    #[serde(default)]
    pub default_cashback_time: Option<String>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI
    /// spec (a string of format `float`).
    #[serde(default)]
    pub default_cashback_percent: Option<String>,
    /// The payment goal in days. Form not measured (`null` in every
    /// measurement); type from the OpenAPI spec (a string).
    #[serde(default)]
    pub default_time_to_pay: Option<String>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub exempt_vat: Option<bool>,
    /// A percentage or an absolute value, as `default_discount_percentage`
    /// says. Form not measured (`null` in every measurement); type from the
    /// OpenAPI spec (a string of format `float`).
    #[serde(default)]
    pub default_discount_amount: Option<String>,
    /// Whether `default_discount_amount` is a percentage; measured `"1"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub default_discount_percentage: Option<bool>,
    /// Measured as text, `null` and `""` (read as `None`).
    #[serde(default, deserialize_with = "string_opt")]
    pub buyer_reference: Option<String>,
    /// Measured `"0"` only.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub government_agency: Option<bool>,
    /// Measured `"1"` only; not in the OpenAPI spec.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub default_show_vat: Option<bool>,
    /// Deprecated in the OpenAPI spec. Form not measured (`null` in every
    /// measurement); type from the spec.
    #[serde(default)]
    pub additional_information: Option<String>,
}

impl Guard for Contact {
    const NAME: &'static str = "Contact";
    fn load_bearing_present(&self) -> bool {
        self.name.is_some()
            || self.surename.is_some()
            || self.familyname.is_some()
            || self.customer_number.is_some()
    }
}

/// A ContactAddress.
///
/// Every key a recording or the shape survey shows filled is a field, and so
/// is every key the OpenAPI spec names. `additionalInformation` is `null` in
/// every measurement and not in the spec (form unmeasured), so it is not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactAddress {
    pub id: ContactAddressId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    /// Measured as a reference and, in the answers of `POST`/`PUT`, as the
    /// full contact; the full object is [`Contact`], returned by its own calls.
    #[serde(default)]
    pub contact: Option<ObjectRef<ContactId>>,
    #[serde(default)]
    pub street: Option<String>,
    #[serde(default)]
    pub zip: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country: Option<ObjectRef<CountryId>>,
    #[serde(default)]
    pub category: Option<ObjectRef<AddressCategoryId>>,
    /// Form not measured (`null` in every measurement); type from the OpenAPI spec.
    #[serde(default)]
    pub name: Option<String>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub name2: Option<String>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub name3: Option<String>,
    /// In no recording or survey object; the form is from the OpenAPI spec.
    #[serde(default)]
    pub name4: Option<String>,
}

impl Guard for ContactAddress {
    const NAME: &'static str = "ContactAddress";
    fn load_bearing_present(&self) -> bool {
        self.street.is_some() || self.zip.is_some() || self.city.is_some()
    }
}

/// A CommunicationWay (e-mail, phone, …) of a contact.
///
/// Every key a recording or the shape survey shows filled is a field, and so
/// is every key the OpenAPI spec names. `additionalInformation` is `null` in
/// every measurement and not in the spec (form unmeasured), so it is not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunicationWay {
    pub id: CommunicationWayId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    #[serde(default)]
    pub contact: Option<ObjectRef<ContactId>>,
    #[serde(default, rename = "type")]
    pub kind: Option<CommunicationWayType>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub key: Option<ObjectRef<CommunicationWayKeyId>>,
    /// `"1"` on the wire.
    #[serde(default, deserialize_with = "deserialize_flag_opt")]
    pub main: Option<bool>,
}

impl Guard for CommunicationWay {
    const NAME: &'static str = "CommunicationWay";
    fn load_bearing_present(&self) -> bool {
        self.kind.is_some() || self.value.is_some()
    }
}
