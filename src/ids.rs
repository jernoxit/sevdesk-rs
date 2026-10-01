//! One id type per resource, so a voucher id can never be passed where a
//! transaction id is meant.

use crate::codec::deserialize_i64;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The `objectName` sevDesk uses for a resource; a reference to it is
/// `{"id": …, "objectName": …}` in a request.
///
/// A trait for type erasure (reason 3): one generic reference shape over many
/// id types, each bringing its own `objectName`.
pub trait Resource {
    const OBJECT_NAME: &'static str;
}

macro_rules! resource_id {
    ($(#[$meta:meta])* $name:ident, $object:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            pub const fn new(id: i64) -> Self {
                Self(id)
            }
            pub const fn get(self) -> i64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        /// Serialized as a JSON integer (sevDesk sends strings, takes integers).
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_i64(self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                deserialize_i64(d).map(Self)
            }
        }

        impl Resource for $name {
            const OBJECT_NAME: &'static str = $object;
        }
    };
}

resource_id!(CheckAccountId, "CheckAccount");
resource_id!(TransactionId, "CheckAccountTransaction");
resource_id!(VoucherId, "Voucher");
resource_id!(VoucherPositionId, "VoucherPos");
resource_id!(InvoiceId, "Invoice");
resource_id!(CreditNoteId, "CreditNote");
resource_id!(DocumentId, "Document");
resource_id!(ContactId, "Contact");
resource_id!(ContactCategoryId, "Category");
resource_id!(
    /// A `Category` of `objectType` `ContactAddress` (`47` is
    /// "Rechnungsanschrift", `43` "Arbeit" on the test account).
    AddressCategoryId,
    "Category"
);
resource_id!(ContactAddressId, "ContactAddress");
resource_id!(CommunicationWayId, "CommunicationWay");
resource_id!(
    /// A `CommunicationWayKey` (`2` is "Arbeit" on the test account).
    CommunicationWayKeyId,
    "CommunicationWayKey"
);
resource_id!(SevUserId, "SevUser");
resource_id!(CountryId, "StaticCountry");
resource_id!(UnityId, "Unity");
resource_id!(
    /// A `PaymentMethod` of the account. The ids are account-specific; find
    /// one with [`crate::SevdeskClient::list_payment_methods`].
    PaymentMethodId,
    "PaymentMethod"
);
resource_id!(TaxRuleId, "TaxRule");
resource_id!(
    /// A DATEV booking account as sevDesk numbers it (`4285` is account 4970
    /// in SKR03) — NOT the account number itself.
    AccountDatevId,
    "AccountDatev"
);

/// A reference to another object: `{"id": "6306017", "objectName": "CheckAccount"}`
/// on the way in, the same with an integer id on the way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRef<I> {
    pub id: I,
}

impl<I> ObjectRef<I> {
    pub const fn new(id: I) -> Self {
        Self { id }
    }
}

impl<I: Resource + Serialize> Serialize for ObjectRef<I> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("ObjectRef", 2)?;
        st.serialize_field("id", &self.id)?;
        st.serialize_field("objectName", I::OBJECT_NAME)?;
        st.end()
    }
}

impl<'de, I: Deserialize<'de>> Deserialize<'de> for ObjectRef<I> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw<I> {
            id: I,
        }
        Raw::deserialize(d).map(|Raw { id }| Self { id })
    }
}
