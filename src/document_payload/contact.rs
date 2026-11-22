use crate::Amount;
use crate::ids::*;
use crate::payload::{ser_amount_opt, ser_iso_date_opt, ser_percent_bp_opt};
use crate::status::ContactStatus;
use serde::{Serialize, Serializer};
use time::Date;

/// Germany in `StaticCountry`.
pub const GERMANY: CountryId = CountryId::new(1);

/// `Category` 3: a customer.
pub const CUSTOMER_CATEGORY: ContactCategoryId = ContactCategoryId::new(3);

/// Who a contact is: an organisation (`name`) or a person (`surename` and
/// `familyname`; sevDesk spells the first one so). Measured: a person has no
/// `name` on the way back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ContactName {
    Organisation {
        name: String,
    },
    Person {
        surename: String,
        familyname: String,
    },
}

/// `POST /Contact`. The named fields are what a contact cannot do without;
/// everything else is in [`Self::options`], set by assignment
/// (`contact.options.vat_number = Some(…)`).
#[derive(Debug, Clone)]
pub struct NewContact {
    pub name: ContactName,
    pub category: ObjectRef<ContactCategoryId>,
    pub options: NewContactOptions,
}

/// The optional fields of a [`NewContact`]. A field left `None` is not sent.
/// Fields measured on the update route (2026-11-22) are taken over for the
/// create route unmeasured.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct NewContactOptions {
    /// The B2B buyer reference; a B2C contact has none.
    pub buyer_reference: Option<String>,
    pub vat_number: Option<String>,
    /// Without one, sevDesk assigns the next free number (measured).
    pub customer_number: Option<String>,
    /// `status`. Without one the contact is created active ([`ContactStatus::Active`], `1000`); measured: stored (2026-11-22; `500` was stored on update).
    pub status: Option<ContactStatus>,
    /// The parent contact; must be an organisation. Measured: stored (2026-11-22).
    pub parent: Option<ObjectRef<ContactId>>,
    /// A non-academic title (sevDesk's spelling). Measured: stored (2026-11-22).
    pub titel: Option<String>,
    /// A description of the contact. Measured: stored (2026-11-22).
    pub description: Option<String>,
    /// An academic title, e.g. `"Dr."`. Measured: stored (2026-11-22).
    pub academic_title: Option<String>,
    /// Gender; measured: `"m"` stored (2026-11-22), other values unmeasured.
    pub gender: Option<String>,
    /// Second name. Measured: stored (2026-11-22).
    pub name2: Option<String>,
    /// Birthday, sent as `yyyy-mm-dd`; reads back as Berlin midnight. Measured: stored (2026-11-22).
    pub birthday: Option<Date>,
    /// IBAN of the contact (`bankAccount`). Measured: stored (2026-11-22).
    pub bank_account: Option<String>,
    /// Bank number (`bankNumber`). Measured: stored (2026-11-22).
    pub bank_number: Option<String>,
    /// Days within which the contact earns a cashback. Measured: stored (2026-11-22).
    pub default_cashback_time: Option<i64>,
    /// Cashback percentage in basis points, sent as a decimal string (`250` is `"2.50"`). Measured: stored (2026-11-22; `"2.5"` was sent).
    pub default_cashback_percent_bp: Option<i64>,
    /// Payment goal in days for every invoice of the contact. Measured: stored (2026-11-22).
    pub default_time_to_pay: Option<i64>,
    /// Tax number. Measured: stored (2026-11-22).
    pub tax_number: Option<String>,
    /// Tax office (sevDesk: only for Greek customers). Measured: stored (2026-11-22).
    pub tax_office: Option<String>,
    /// Exempt from VAT; sent as a JSON bool. Measured: stored (2026-11-22).
    pub exempt_vat: Option<bool>,
    /// Default discount as an exact decimal string. Whether it is a percentage is `defaultDiscountPercentage`, which is not taken in (measurement: same value, no statement). Measured: stored (2026-11-22).
    pub default_discount_amount: Option<Amount>,
    /// Government agency; sent as a JSON bool. Measured: stored (2026-11-22).
    pub government_agency: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewContactWire {
    #[serde(flatten)]
    name: ContactName,
    category: ObjectRef<ContactCategoryId>,
    status: ContactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    buyer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vat_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<ObjectRef<ContactId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    titel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    academic_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name2: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_iso_date_opt"
    )]
    birthday: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bank_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bank_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_cashback_time: Option<i64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "defaultCashbackPercent",
        serialize_with = "ser_percent_bp_opt"
    )]
    default_cashback_percent_bp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_time_to_pay: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tax_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tax_office: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exempt_vat: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_amount_opt"
    )]
    default_discount_amount: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    government_agency: Option<bool>,
}

impl From<NewContact> for NewContactWire {
    fn from(contact: NewContact) -> Self {
        let NewContact {
            name,
            category,
            options,
        } = contact;
        let NewContactOptions {
            buyer_reference,
            vat_number,
            customer_number,
            status,
            parent,
            titel,
            description,
            academic_title,
            gender,
            name2,
            birthday,
            bank_account,
            bank_number,
            default_cashback_time,
            default_cashback_percent_bp,
            default_time_to_pay,
            tax_number,
            tax_office,
            exempt_vat,
            default_discount_amount,
            government_agency,
        } = options;
        Self {
            name,
            category,
            status: status.unwrap_or(ContactStatus::Active),
            buyer_reference,
            vat_number,
            customer_number,
            parent,
            titel,
            description,
            academic_title,
            gender,
            name2,
            birthday,
            bank_account,
            bank_number,
            default_cashback_time,
            default_cashback_percent_bp,
            default_time_to_pay,
            tax_number,
            tax_office,
            exempt_vat,
            default_discount_amount,
            government_agency,
        }
    }
}

impl Serialize for NewContact {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        NewContactWire::from(self.clone()).serialize(s)
    }
}

impl NewContact {
    /// An active customer ([`CUSTOMER_CATEGORY`]) without the optional fields.
    pub fn new(name: ContactName) -> Self {
        Self {
            name,
            category: ObjectRef::new(CUSTOMER_CATEGORY),
            options: NewContactOptions::default(),
        }
    }
}

/// `PUT /Contact/{id}`. A partial update is measured: fields left `None`
/// stay as they are. Measured 2026-10-01: `name` on a person sets the name and
/// leaves `surename` and `familyname` as they are; the contact does not
/// become an organisation. All fields are optional and the type has no
/// `#[non_exhaustive]`, so `..Default::default()` keeps working.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub familyname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_number: Option<String>,
    /// The category. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<ObjectRef<ContactCategoryId>>,
    /// `status`. Without one the contact is created active ([`ContactStatus::Active`], `1000`); measured: stored (2026-11-22; `500` was stored on update).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ContactStatus>,
    /// The parent contact; must be an organisation. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<ObjectRef<ContactId>>,
    /// A non-academic title (sevDesk's spelling). Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub titel: Option<String>,
    /// A description of the contact. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An academic title, e.g. `"Dr."`. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub academic_title: Option<String>,
    /// Gender; measured: `"m"` stored (2026-11-22), other values unmeasured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    /// Second name. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name2: Option<String>,
    /// Birthday, sent as `yyyy-mm-dd`; reads back as Berlin midnight. Measured: stored (2026-11-22).
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_iso_date_opt"
    )]
    pub birthday: Option<Date>,
    /// IBAN of the contact (`bankAccount`). Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account: Option<String>,
    /// Bank number (`bankNumber`). Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_number: Option<String>,
    /// Days within which the contact earns a cashback. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_cashback_time: Option<i64>,
    /// Cashback percentage in basis points, sent as a decimal string (`250` is `"2.50"`). Measured: stored (2026-11-22; `"2.5"` was sent).
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "defaultCashbackPercent",
        serialize_with = "ser_percent_bp_opt"
    )]
    pub default_cashback_percent_bp: Option<i64>,
    /// Payment goal in days for every invoice of the contact. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_time_to_pay: Option<i64>,
    /// Tax number. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_number: Option<String>,
    /// Tax office (sevDesk: only for Greek customers). Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_office: Option<String>,
    /// Exempt from VAT; sent as a JSON bool. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_vat: Option<bool>,
    /// Default discount as an exact decimal string. Whether it is a percentage is `defaultDiscountPercentage`, which is not taken in (measurement: same value, no statement). Measured: stored (2026-11-22).
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_amount_opt"
    )]
    pub default_discount_amount: Option<Amount>,
    /// Government agency; sent as a JSON bool. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub government_agency: Option<bool>,
}

/// `POST /ContactAddress`. The named fields are required; the rest is in
/// [`Self::options`].
#[derive(Debug, Clone)]
pub struct NewContactAddress {
    pub contact: ObjectRef<ContactId>,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub country: ObjectRef<CountryId>,
    pub category: ObjectRef<AddressCategoryId>,
    pub options: NewContactAddressOptions,
}

/// The optional fields of a [`NewContactAddress`]. A field left `None` is not
/// sent. `name2` to `name4` of the spec are ignored by sevDesk (measured
/// 2026-11-22) and not taken in.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct NewContactAddressOptions {
    /// The name in the address. Measured: stored (2026-11-22, on update).
    pub name: Option<String>,
}

#[derive(Serialize)]
struct NewContactAddressWire {
    contact: ObjectRef<ContactId>,
    street: String,
    zip: String,
    city: String,
    country: ObjectRef<CountryId>,
    category: ObjectRef<AddressCategoryId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl From<NewContactAddress> for NewContactAddressWire {
    fn from(address: NewContactAddress) -> Self {
        let NewContactAddress {
            contact,
            street,
            zip,
            city,
            country,
            category,
            options,
        } = address;
        let NewContactAddressOptions { name } = options;
        Self {
            contact,
            street,
            zip,
            city,
            country,
            category,
            name,
        }
    }
}

impl Serialize for NewContactAddress {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        NewContactAddressWire::from(self.clone()).serialize(s)
    }
}

/// `PUT /ContactAddress/{id}`. A partial update (measured 2026-10-01). No
/// `#[non_exhaustive]`, so `..Default::default()` keeps working.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ContactAddressUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<ObjectRef<CountryId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<ObjectRef<AddressCategoryId>>,
    /// The name in the address. Measured: stored (2026-11-22).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `POST /CommunicationWay`.
#[derive(Debug, Clone, Serialize)]
pub struct NewCommunicationWay {
    pub contact: ObjectRef<ContactId>,
    #[serde(rename = "type")]
    kind: crate::CommunicationWayType,
    pub value: String,
    pub key: ObjectRef<CommunicationWayKeyId>,
    /// The main way of its type for the contact.
    pub main: bool,
}

impl NewCommunicationWay {
    /// An e-mail address (`type` `EMAIL`).
    pub fn email(
        contact: ContactId,
        value: impl Into<String>,
        key: CommunicationWayKeyId,
        main: bool,
    ) -> Self {
        Self {
            contact: ObjectRef::new(contact),
            kind: crate::CommunicationWayType::Email,
            value: value.into(),
            key: ObjectRef::new(key),
            main,
        }
    }
}

/// `PUT /CommunicationWay/{id}`. A partial update (measured 2026-10-01). No
/// `#[non_exhaustive]`, so `..Default::default()` keeps working.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CommunicationWayUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<ObjectRef<CommunicationWayKeyId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<bool>,
    /// The type of the way. Measured: `PHONE` stored (2026-11-22).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<crate::CommunicationWayType>,
}
