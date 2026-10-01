//! The one error type of this crate. It says WHAT happened; what to do about
//! it (retry, alert, give up) is the caller's decision.

use serde::Deserialize;

/// Error codes sevDesk sends in `error.code`, as measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownCode {
    /// 151: no such object.
    NotFound,
    /// 159: the object is no longer a draft (invoice or voucher in status 200
    /// or above, a booked transaction), so it cannot be deleted (HTTP 409).
    NotDeletable,
    /// 170: a credit note stands beside the invoice, so it can be neither
    /// cancelled nor reset to open (HTTP 422).
    CreditNoteExists,
    /// 390: cancelling a partly paid invoice (HTTP 200 with an `error` body):
    /// book a credit note manually instead.
    PartlyPaid,
    /// 421: the invoice is already cancelled (HTTP 422).
    AlreadyCancelled,
    /// 455: the credit notes' sum would exceed the invoice they refer to.
    CreditSumExceedsInvoice,
    /// 456: a credit note's tax rate differs from its origin invoice's. Also
    /// the answer to an `UNDERACHIEVEMENT` credit note without `refSrcInvoice`.
    TaxRateDiffersFromOrigin,
    /// 750: the invoice is already paid.
    InvoiceAlreadyPaid,
    /// 751: the voucher is already paid (booking over the total is refused).
    VoucherAlreadyPaid,
    /// 830: the e-invoice cannot be generated because fields are missing
    /// (HTTP 400). The error message lists them.
    EInvoiceFieldsMissing,
    /// 7100: enshrining would give the invoice a negative sum.
    NegativeInvoiceSum,
}

impl KnownCode {
    pub const fn from_code(code: i64) -> Option<Self> {
        match code {
            151 => Some(Self::NotFound),
            159 => Some(Self::NotDeletable),
            170 => Some(Self::CreditNoteExists),
            390 => Some(Self::PartlyPaid),
            421 => Some(Self::AlreadyCancelled),
            455 => Some(Self::CreditSumExceedsInvoice),
            456 => Some(Self::TaxRateDiffersFromOrigin),
            750 => Some(Self::InvoiceAlreadyPaid),
            751 => Some(Self::VoucherAlreadyPaid),
            830 => Some(Self::EInvoiceFieldsMissing),
            7100 => Some(Self::NegativeInvoiceSum),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SevdeskError {
    /// sevDesk answered with an `error` object — with any HTTP status, also 200.
    #[error("sevDesk error (HTTP {status}, code {code:?}): {message}")]
    Api {
        /// The HTTP status, or the body's `responseCode` when the HTTP status was 2xx.
        status: u16,
        code: Option<i64>,
        message: String,
    },
    /// 404 with code 151: the object does not exist.
    #[error("sevDesk object not found: {message}")]
    NotFound { message: String },
    /// No usable answer: connection, timeout, unreadable body.
    #[error("sevDesk transport: {0}")]
    Transport(#[from] reqwest::Error),
    /// The body is valid JSON but not the shape this crate reads.
    #[error("sevDesk response does not decode ({context}): {source}")]
    Decode {
        context: &'static str,
        source: serde_json::Error,
    },
    /// The body decoded but none of the fields this crate relies on is there:
    /// sevDesk changed shape, and a defaulted object would be a lie.
    #[error("sevDesk response drifted ({context}): {detail}")]
    DriftGuard {
        context: &'static str,
        detail: String,
    },
    /// HTTP 429 persisted through every retry.
    #[error("sevDesk rate limit persisted after {attempts} attempts")]
    RateLimited { attempts: u32 },
    /// The request was refused before it was sent.
    #[error("invalid sevDesk request: {0}")]
    InvalidRequest(String),
}

impl SevdeskError {
    /// The `error.code` of an [`SevdeskError::Api`] or the 151 of a `NotFound`, as data.
    pub fn known_code(&self) -> Option<KnownCode> {
        match self {
            Self::NotFound { .. } => Some(KnownCode::NotFound),
            Self::Api { code: Some(c), .. } => KnownCode::from_code(*c),
            _ => None,
        }
    }

    /// 422 "You can only create transactions on online checkaccounts": the
    /// account is an offline one, and no offline account takes transactions.
    pub fn is_offline_account(&self) -> bool {
        matches!(self, Self::Api { status: 422, message, .. } if message.contains("online checkaccounts"))
    }
}

/// `error` object of a body.
#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorBody {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default, deserialize_with = "crate::codec::deserialize_i64_opt")]
    pub code: Option<i64>,
    #[serde(
        default,
        rename = "responseCode",
        deserialize_with = "crate::codec::deserialize_i64_opt"
    )]
    pub response_code: Option<i64>,
    #[serde(default)]
    data: Option<ErrorData>,
}

/// `error.data`: an empty list, a missing-information object (code 830), or
/// something else this crate does not read.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ErrorData {
    Missing {
        #[serde(rename = "missing information")]
        missing: Vec<String>,
    },
    Other(serde::de::IgnoredAny),
}

impl ApiErrorBody {
    pub(crate) fn into_error(self, http_status: u16) -> SevdeskError {
        let mut message = self.message.unwrap_or_default();
        if let Some(ErrorData::Missing { missing }) = &self.data {
            message = format!("{message} (missing: {})", missing.join(", "));
        }
        let status = if http_status >= 400 {
            http_status
        } else {
            self.response_code
                .and_then(|c| u16::try_from(c).ok())
                .unwrap_or(http_status)
        };
        if status == 404 && self.code == Some(151) {
            SevdeskError::NotFound { message }
        } else {
            SevdeskError::Api {
                status,
                code: self.code,
                message,
            }
        }
    }
}
