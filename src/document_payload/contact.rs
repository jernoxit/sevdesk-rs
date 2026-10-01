use crate::ids::*;
use serde::Serialize;

/// Germany in `StaticCountry`.
pub const GERMANY: CountryId = CountryId::new(1);

/// `Category` 3: a customer.
pub const CUSTOMER_CATEGORY: ContactCategoryId = ContactCategoryId::new(3);

/// `status` 1000: an active contact.
const ACTIVE: i64 = 1000;

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

/// `POST /Contact`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewContact {
    #[serde(flatten)]
    pub name: ContactName,
    pub category: ObjectRef<ContactCategoryId>,
    status: i64,
    /// The B2B buyer reference; a B2C contact has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
    /// Without one, sevDesk assigns the next free number (measured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_number: Option<String>,
}

impl NewContact {
    /// An active customer ([`CUSTOMER_CATEGORY`]) without the optional fields.
    pub fn new(name: ContactName) -> Self {
        Self {
            name,
            category: ObjectRef::new(CUSTOMER_CATEGORY),
            status: ACTIVE,
            buyer_reference: None,
            vat_number: None,
            customer_number: None,
        }
    }
}

/// `PUT /Contact/{id}`. A partial update is measured: fields left `None`
/// stay as they are. Measured 2026-10-01: `name` on a person sets the name and
/// leaves `surename` and `familyname` as they are; the contact does not
/// become an organisation.
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
}

/// `POST /ContactAddress`.
#[derive(Debug, Clone, Serialize)]
pub struct NewContactAddress {
    pub contact: ObjectRef<ContactId>,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub country: ObjectRef<CountryId>,
    pub category: ObjectRef<AddressCategoryId>,
}

/// `PUT /ContactAddress/{id}`. A partial update (measured 2026-10-01).
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

/// `PUT /CommunicationWay/{id}`. A partial update (measured 2026-10-01).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CommunicationWayUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<ObjectRef<CommunicationWayKeyId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<bool>,
}
