//! **Typed client for the sevDesk REST API**: sevDesk vocabulary, typed
//! requests and responses, transport. No business logic.
//!
//! Shaped by measurements against a real account; where they contradict
//! sevDesk's OpenAPI spec, the measurement wins:
//!
//! - the auto-mapping field is `autoMapTransaction` (singular);
//! - `customerInternalNote` is spelled correctly here, the spec's
//!   `customerIntenalNote` is ignored by the server (it answers with everything);
//! - filters on `paymtPurpose` and `descriptionLike` match SUBSTRINGS;
//! - a deleted transaction is still returned by id, with `deleted_at` set;
//! - a booking amount carries the sign of the transaction;
//! - `saveVoucher` updates only drafts (400, code 90400) and vouchers delete only
//!   as drafts (409, code 159); transactions delete only in status 100 (409, 159);
//! - every body is checked for an `error` object, also on HTTP 200;
//! - `currency` came on every check account, voucher, invoice and credit note
//!   (the spec allows `null` on vouchers and credit notes), so one without it
//!   is a decode error.
//!
//! Money is [`Amount`] (integer cents): read exactly from strings or numbers,
//! written as exact decimal strings. No floats anywhere. An object that
//! carries its currency on the wire exposes it as [`CurrencyCode`] beside the
//! amounts; the others name whose currency their amounts are in.
//!
//! The library reads no environment: everything comes in through
//! [`SevdeskClientConfig`].
//!
//! Layout: `wire` (what comes in), `payload` and `query` (what goes out),
//! `client` (transport), `pacing` (AIMD request spacing), `endpoints/` (one file per resource).

mod amount;
mod client;
mod codec;
mod currency;
mod document_payload;
mod endpoints;
mod error;
mod ids;
mod pacing;
mod payload;
mod query;
mod status;
mod wire;

pub use amount::Amount;
pub use client::{
    ApiToken, DEFAULT_BASE_URL, DEFAULT_X_VERSION, SevdeskClient, SevdeskClientConfig,
};
pub use codec::{format_decimal, parse_amount};
pub use currency::{CurrencyCode, InvalidCurrencyCode};
pub use document_payload::*;
pub use error::{KnownCode, SevdeskError};
pub use ids::*;
pub use pacing::Pacing;
pub use payload::*;
pub use query::{CreditNoteQuery, InvoiceQuery, Page, TransactionQuery, VoucherQuery};
pub use status::*;
pub use wire::{
    AllowedTaxRule, BookedDocument, CheckAccount, CommunicationWay, Contact, ContactAddress,
    CreditNote, DocumentPdf, Invoice, PaymentMethod, ReceiptGuidance, SavedCreditNote,
    SavedInvoice, SavedVoucher, SevSequence, SevUser, Transaction, TransactionLogEntry,
    UploadedFile, Voucher, VoucherPosition,
};
