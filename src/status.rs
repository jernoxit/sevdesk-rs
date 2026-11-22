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
    /// Voucher (expense document) status. `750` is measured although the
    /// OpenAPI enumerations leave it out.
    VoucherStatus {
        Draft = 50,
        Open = 100,
        /// `150`, "Transferred"; known only from the OpenAPI spec prose.
        Transferred = 150,
        PartiallyPaid = 750,
        Paid = 1000,
    }
);

coded_enum!(
    /// Invoice status (`100` is a draft for invoices, unlike for vouchers).
    InvoiceStatus {
        /// Deactivated recurring invoice; known only from the OpenAPI spec.
        DeactivatedRecurring = 50,
        Draft = 100,
        Open = 200,
        PartiallyPaid = 750,
        Paid = 1000,
    }
);

coded_enum!(
    /// `status` of a contact. `1000` is measured on every contact; `100` and
    /// `500` are known only from the OpenAPI spec prose (no enumeration).
    ContactStatus {
        /// `100`; known only from the OpenAPI spec.
        Lead = 100,
        /// `500`; known only from the OpenAPI spec.
        Pending = 500,
        Active = 1000,
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
    ///
    /// `350` is known only from the OpenAPI response spec; its meaning is unknown.
    TransactionStatus {
        Created = 100,
        Linked = 200,
        Private = 300,
        /// `350`; known only from the OpenAPI response spec.
        Unknown350 = 350,
        Booked = 400,
    }
);

named_enum!(
    /// `invoiceType`: `RE` invoice, `SR` cancellation invoice (both measured);
    /// the rest are known only from the OpenAPI spec, anything else is `Other`.
    InvoiceType {
        Invoice = "RE",
        Cancellation = "SR",
        /// `WKR`, recurring invoice; known only from the OpenAPI spec.
        Recurring = "WKR",
        /// `MA`, reminder (Mahnung); known only from the OpenAPI spec.
        Reminder = "MA",
        /// `TR`, partial invoice (Teilrechnung); known only from the OpenAPI spec.
        Partial = "TR",
        /// `AR`, advance invoice (Abschlagsrechnung); known only from the OpenAPI spec.
        Advance = "AR",
        /// `ER`, final invoice (Schlussrechnung); known only from the OpenAPI spec.
        Final = "ER",
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

named_enum!(
    /// `importType` as a check account answer carries it. The outgoing
    /// [`ImportType`] stays `Copy` and has no `Other`; this read form is
    /// tolerant. An account without file import has `null`, not a variant.
    CheckAccountImportType {
        /// `CSV`; measured.
        Csv = "CSV",
        /// `MT940`; known only from the OpenAPI spec.
        Mt940 = "MT940",
    }
);

named_enum!(
    /// `type` of a check account: `online` and `offline` are measured,
    /// `register` is known only from the OpenAPI spec.
    CheckAccountType {
        Online = "online",
        Offline = "offline",
        /// `register`; known only from the OpenAPI spec.
        Register = "register",
    }
);

coded_enum!(
    /// `status` of a check account. `100` is measured on every account, `0`
    /// is known only from the OpenAPI spec; the names are inferred.
    CheckAccountStatus {
        /// `0`; known only from the OpenAPI spec.
        Inactive = 0,
        Active = 100,
    }
);

/// How `sendBy` "sends" a document; `Vpdf` only marks it as sent (no mail, no
/// print) and finalizes it, which is what gives an invoice its number. Whether it
/// also enshrines depends on the account setting (see [`crate::SendBy::FINALIZE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SendType {
    #[serde(rename = "VPDF")]
    Vpdf,
}

named_enum!(
    /// `sendType` as an invoice or credit note answer carries it. The outgoing
    /// [`SendType`] stays `Copy` and has no `Other`; this read form is tolerant.
    DocumentSendType {
        /// `"0"`, measured on drafts that were never sent (the meaning is inferred).
        Unset = "0",
        /// Measured; the document was marked as sent (`VPDF`).
        Vpdf = "VPDF",
        /// `VPR`; known only from the OpenAPI spec.
        Vpr = "VPR",
        /// `VM`; known only from the OpenAPI spec.
        Vm = "VM",
        /// `VP`; known only from the OpenAPI spec.
        Vp = "VP",
    }
);

named_enum!(
    /// `taxType` of a document. Every recording has `null`, so all variants
    /// are known only from the OpenAPI spec (`default|eu|noteu|custom`).
    TaxType {
        Default = "default",
        Eu = "eu",
        NotEu = "noteu",
        Custom = "custom",
    }
);

named_enum!(
    /// `bookingCategory` as a credit-note answer carries it. The outgoing
    /// [`CreditNoteCategory`] stays `Copy` and has no `Other`; this read form
    /// is tolerant.
    CreditNoteBookingCategory {
        /// `PROVISION`; measured.
        Provision = "PROVISION",
        /// `UNDERACHIEVEMENT`; measured.
        Underachievement = "UNDERACHIEVEMENT",
        /// `ROYALTY_ASSIGNED`; seen in the shape survey, not in a fixture.
        RoyaltyAssigned = "ROYALTY_ASSIGNED",
        /// `ROYALTY_UNASSIGNED`; known only from the OpenAPI spec.
        RoyaltyUnassigned = "ROYALTY_UNASSIGNED",
        /// `ACCOUNTING_TYPE`; known only from the OpenAPI spec.
        AccountingType = "ACCOUNTING_TYPE",
    }
);

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
    /// The `type` of a communication way. Only `EMAIL` is measured; the rest
    /// is known only from the OpenAPI spec.
    CommunicationWayType {
        Email = "EMAIL",
        /// `PHONE`; known only from the OpenAPI spec.
        Phone = "PHONE",
        /// `WEB`; known only from the OpenAPI spec.
        Web = "WEB",
        /// `MOBILE`; known only from the OpenAPI spec.
        Mobile = "MOBILE",
    }
);

named_enum!(
    /// `recurringInterval` of a recurring voucher, an ISO 8601 duration. Every
    /// measurement has `null`, so all variants are known only from the OpenAPI
    /// spec; the names read the durations (`P0Y1M0W` is one month).
    RecurringInterval {
        /// `P0Y0M1W`; known only from the OpenAPI spec.
        Weekly = "P0Y0M1W",
        /// `P0Y0M2W`; known only from the OpenAPI spec.
        EveryTwoWeeks = "P0Y0M2W",
        /// `P0Y1M0W`; known only from the OpenAPI spec.
        Monthly = "P0Y1M0W",
        /// `P0Y3M0W`; known only from the OpenAPI spec.
        Quarterly = "P0Y3M0W",
        /// `P0Y6M0W`; known only from the OpenAPI spec.
        HalfYearly = "P0Y6M0W",
        /// `P1Y0M0W`; known only from the OpenAPI spec.
        Yearly = "P1Y0M0W",
        /// `P2Y0M0W`; known only from the OpenAPI spec.
        EveryTwoYears = "P2Y0M0W",
        /// `P3Y0M0W`; known only from the OpenAPI spec.
        EveryThreeYears = "P3Y0M0W",
        /// `P4Y0M0W`; known only from the OpenAPI spec.
        EveryFourYears = "P4Y0M0W",
        /// `P5Y0M0W`; known only from the OpenAPI spec.
        EveryFiveYears = "P5Y0M0W",
    }
);

named_enum!(
    /// An entry of `allowedReceiptTypes` of a receipt guidance. `EXPENSE` and
    /// `REVENUE` are measured; the OpenAPI spec names only the example
    /// `EXPENSE` and no enumeration.
    ReceiptType {
        Expense = "EXPENSE",
        Revenue = "REVENUE",
    }
);
