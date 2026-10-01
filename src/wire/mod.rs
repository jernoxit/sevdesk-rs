//! What sevDesk sends: tolerant `Deserialize` types.
//!
//! Every field except the id and the currency is optional and unknown fields
//! are ignored — sevDesk grows fields. The counterweight is the drift guard: an
//! object of which NONE of the load-bearing fields is present is an error
//! ([`crate::SevdeskError::DriftGuard`]), never a defaulted object.
//!
//! Amounts are [`crate::Amount`], read exactly from strings or numbers. An object
//! that carries `currency` on the wire has it as a required [`crate::CurrencyCode`]:
//! missing or `null` is [`crate::SevdeskError::Decode`], never a default.
//! Measured with EUR only: whether a document in another currency reports
//! `sumGross` in that currency or in the account's (sevDesk also has
//! `sum*ForeignCurrency`, not read here) is open.
//! Percentages are basis points (`19` % is `1900`).

mod check_account;
mod contact;
mod credit_note;
mod document_file;
mod guard;
mod invoice;
mod misc;
mod transaction;
mod voucher;

pub use check_account::*;
pub use contact::*;
pub use credit_note::*;
pub use document_file::DocumentPdf;
pub(crate) use document_file::xml_from_body;
pub(crate) use guard::Guard;
pub use invoice::*;
pub use misc::*;
pub use transaction::*;
pub use voucher::*;
