//! Status codes and type codes. Each enum has an `Other` arm: sevDesk adds
//! values without notice, and an unknown one must stay readable (and
//! visible) instead of failing the whole response.

use crate::codec::deserialize_i64;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! coded_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $var:ident = $code:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $var,)+
            /// A code this crate does not know.
            Other(i64),
        }

        impl $name {
            pub const fn code(self) -> i64 {
                match self { $(Self::$var => $code,)+ Self::Other(code) => code }
            }
            pub const fn from_code(code: i64) -> Self {
                match code { $($code => Self::$var,)+ code => Self::Other(code) }
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_i64(self.code())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                deserialize_i64(d).map(Self::from_code)
            }
        }
    };
}

macro_rules! named_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $var:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $var,)+
            /// A value this crate does not know.
            Other(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self { $(Self::$var => $text,)+ Self::Other(text) => text }
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = String::deserialize(d)?;
                Ok(match text.as_str() {
                    $($text => Self::$var,)+
                    _ => Self::Other(text),
                })
            }
        }
    };
}

coded_enum!(
    /// Voucher (expense document) status.
    VoucherStatus {
        Draft = 50,
        Open = 100,
        PartiallyPaid = 750,
        Paid = 1000,
    }
);

coded_enum!(
    /// Invoice status (`100` is a draft for invoices, unlike for vouchers).
    InvoiceStatus {
        Draft = 100,
        Open = 200,
        PartiallyPaid = 750,
        Paid = 1000,
    }
);

coded_enum!(
    /// Credit note status; measured: `200` open, `1000` paid.
    CreditNoteStatus {
        Draft = 100,
        Open = 200,
        PartiallyPaid = 750,
        Paid = 1000,
    }
);

coded_enum!(
    /// Check account transaction status.
    TransactionStatus {
        Created = 100,
        Linked = 200,
        Private = 300,
        Booked = 400,
    }
);

named_enum!(
    /// `invoiceType`: `RE` invoice, `SR` cancellation invoice; the rest is `Other`.
    InvoiceType {
        Invoice = "RE",
        Cancellation = "SR",
    }
);

named_enum!(
    /// `creditDebit` of a voucher: `C` an expense, `D` an income.
    CreditDebit {
        Credit = "C",
        Debit = "D",
    }
);

named_enum!(
    /// `voucherType`: `VOU` a normal voucher, `RV` a recurring one.
    VoucherType {
        Normal = "VOU",
        Recurring = "RV",
    }
);

/// The `type` of a `bookAmount` call. Outgoing only, so no `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BookingType {
    #[serde(rename = "FULL_PAYMENT")]
    FullPayment,
    /// Partial booking.
    N,
    /// Reduced amount because of a discount (Skonto).
    #[serde(rename = "CB")]
    Discount,
    #[serde(rename = "CF")]
    CurrencyFluctuation,
    /// Difference for another reason.
    O,
    /// Higher amount because of reminder charges.
    #[serde(rename = "OF")]
    ReminderCharges,
    /// Reduced amount because of monetary traffic costs.
    #[serde(rename = "MTC")]
    MonetaryTrafficCosts,
}

/// The `importType` of a file-import check account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImportType {
    #[serde(rename = "CSV")]
    Csv,
    #[serde(rename = "MT940")]
    Mt940,
}

/// How `sendBy` "sends" a document; `Vpdf` only marks it as sent (no mail, no
/// print) and enshrines it, which is what gives an invoice its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SendType {
    #[serde(rename = "VPDF")]
    Vpdf,
}

/// `bookingCategory` of a credit note. Outgoing only, so no `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CreditNoteCategory {
    /// Takes tax rule 9 and no invoice reference (measured).
    #[serde(rename = "PROVISION")]
    Provision,
    /// Refers to an invoice (`refSrcInvoice`) and takes its tax rate (measured).
    #[serde(rename = "UNDERACHIEVEMENT")]
    Underachievement,
}

/// A value of the `language` parameter of a document. Outgoing only, so no
/// `Other`; sevDesk lists more languages than these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DocumentLanguage {
    /// Measured: sevDesk stores `en` when sent `en_US`.
    #[serde(rename = "en_US")]
    EnUs,
    #[serde(rename = "de")]
    De,
    #[serde(rename = "de_AT")]
    DeAt,
    #[serde(rename = "de_CH")]
    DeCh,
}

named_enum!(
    /// The `type` of a communication way. Only `EMAIL` is measured.
    CommunicationWayType {
        Email = "EMAIL",
    }
);
