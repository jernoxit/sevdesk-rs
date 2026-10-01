use super::Guard;
use crate::ids::*;
use crate::status::CommunicationWayType;
use serde::Deserialize;

/// A Contact (customer). An organisation has `name`, a person `surename` and
/// `familyname` (measured: `name` is `null` for a person).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: ContactId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub surename: Option<String>,
    #[serde(default)]
    pub familyname: Option<String>,
    /// A string on the wire (`"1000"`).
    #[serde(default)]
    pub customer_number: Option<String>,
    #[serde(default)]
    pub buyer_reference: Option<String>,
    #[serde(default)]
    pub vat_number: Option<String>,
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactAddress {
    pub id: ContactAddressId,
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
}

impl Guard for ContactAddress {
    const NAME: &'static str = "ContactAddress";
    fn load_bearing_present(&self) -> bool {
        self.street.is_some() || self.zip.is_some() || self.city.is_some()
    }
}

/// A CommunicationWay (e-mail, phone, …) of a contact.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunicationWay {
    pub id: CommunicationWayId,
    #[serde(default)]
    pub contact: Option<ObjectRef<ContactId>>,
    #[serde(default, rename = "type")]
    pub kind: Option<CommunicationWayType>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub key: Option<ObjectRef<CommunicationWayKeyId>>,
    /// `"1"` on the wire.
    #[serde(default, deserialize_with = "crate::codec::deserialize_flag_opt")]
    pub main: Option<bool>,
}

impl Guard for CommunicationWay {
    const NAME: &'static str = "CommunicationWay";
    fn load_bearing_present(&self) -> bool {
        self.kind.is_some() || self.value.is_some()
    }
}
