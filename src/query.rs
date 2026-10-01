//! Query parameters of the list endpoints, as typed values.
//!
//! sevDesk's filters are name-sensitive and fail silently: a misspelled or
//! unknown filter is IGNORED and the call answers with everything (measured
//! for `customerIntenalNote`, the spec's typo, and for `externalId`). Every
//! key here is a measured one.

use crate::error::SevdeskError;
use crate::ids::*;
use crate::status::*;
use time::OffsetDateTime;

pub(crate) type Pairs = Vec<(&'static str, String)>;

/// `limit` (1..=1000) and `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    limit: u32,
    offset: u32,
}

impl Page {
    pub const MAX_LIMIT: u32 = 1000;

    pub fn new(limit: u32, offset: u32) -> Result<Self, SevdeskError> {
        if !(1..=Self::MAX_LIMIT).contains(&limit) {
            return Err(SevdeskError::InvalidRequest(format!(
                "limit must be 1..={}, got {limit}",
                Self::MAX_LIMIT
            )));
        }
        Ok(Self { limit, offset })
    }

    pub(crate) fn push_to(self, pairs: &mut Pairs) {
        Self::push(Some(self), pairs);
    }

    fn push(page: Option<Self>, pairs: &mut Pairs) {
        if let Some(Self { limit, offset }) = page {
            pairs.push(("limit", limit.to_string()));
            pairs.push(("offset", offset.to_string()));
        }
    }
}

/// `GET /CheckAccountTransaction`. The list omits deleted transactions.
#[derive(Debug, Clone)]
pub struct TransactionQuery {
    pub check_account: CheckAccountId,
    /// SUBSTRING match at sevDesk: compare exactly on the result.
    pub paymt_purpose: Option<String>,
    /// Sent as a Unix timestamp; `startDate`/`endDate` accept it like ISO dates.
    /// Measured on the sevDesk test tenant (2026-10-01): the `startDate` and
    /// `endDate` filter cuts by `entryDate`, NOT by `valueDate`. Three
    /// transactions with the same `valueDate` and `entryDate`s of 01.10.,
    /// 10.10. and 20.10., queried with the window 05.-15.10., returned only
    /// the 10.10. one.
    pub start_date: Option<OffsetDateTime>,
    /// Cuts by `entryDate`, like [`Self::start_date`].
    pub end_date: Option<OffsetDateTime>,
    /// Measured: `true` returns only booked transactions, but `false` is
    /// IGNORED and returns everything. To ask for the open ones use
    /// [`Self::status`].
    pub is_booked: Option<bool>,
    /// Measured: `status=100` returns only transactions in that status.
    pub status: Option<TransactionStatus>,
    pub page: Option<Page>,
}

impl TransactionQuery {
    pub fn new(check_account: CheckAccountId) -> Self {
        Self {
            check_account,
            paymt_purpose: None,
            start_date: None,
            end_date: None,
            is_booked: None,
            status: None,
            page: None,
        }
    }

    pub(crate) fn pairs(&self) -> Pairs {
        let mut p: Pairs = vec![
            ("checkAccount[id]", self.check_account.to_string()),
            (
                "checkAccount[objectName]",
                CheckAccountId::OBJECT_NAME.into(),
            ),
        ];
        if let Some(v) = &self.paymt_purpose {
            p.push(("paymtPurpose", v.clone()));
        }
        if let Some(v) = self.start_date {
            p.push(("startDate", v.unix_timestamp().to_string()));
        }
        if let Some(v) = self.end_date {
            p.push(("endDate", v.unix_timestamp().to_string()));
        }
        if let Some(v) = self.is_booked {
            p.push(("isBooked", v.to_string()));
        }
        if let Some(v) = self.status {
            p.push(("status", v.code().to_string()));
        }
        Page::push(self.page, &mut p);
        p
    }
}

/// `GET /Voucher`.
#[derive(Debug, Clone, Default)]
pub struct VoucherQuery {
    /// SUBSTRING match at sevDesk: compare exactly on the result.
    pub description_like: Option<String>,
    pub status: Option<VoucherStatus>,
    pub page: Option<Page>,
}

impl VoucherQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let mut p = Pairs::new();
        if let Some(v) = &self.description_like {
            p.push(("descriptionLike", v.clone()));
        }
        if let Some(v) = self.status {
            p.push(("status", v.code().to_string()));
        }
        Page::push(self.page, &mut p);
        p
    }
}

/// `GET /Invoice`.
#[derive(Debug, Clone, Default)]
pub struct InvoiceQuery {
    /// EXACT match; hits the original AND its cancellation invoice.
    pub customer_internal_note: Option<String>,
    /// Compare exactly on the result: not measured to be an exact match.
    pub invoice_number: Option<String>,
    pub status: Option<InvoiceStatus>,
    /// Only invoices changed at or after this instant, sent as Unix seconds.
    /// [`crate::SevdeskClient::list_invoices`] checks every answer against it
    /// (a filter sevDesk does not understand is ignored and returns everything).
    /// `invoiceType` is not offered: it is not measured as a server filter, so
    /// filter on [`crate::Invoice::invoice_type`].
    pub update_after: Option<OffsetDateTime>,
    pub page: Option<Page>,
}

impl InvoiceQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let mut p = Pairs::new();
        if let Some(v) = &self.customer_internal_note {
            p.push(("customerInternalNote", v.clone()));
        }
        if let Some(v) = &self.invoice_number {
            p.push(("invoiceNumber", v.clone()));
        }
        if let Some(v) = self.status {
            p.push(("status", v.code().to_string()));
        }
        if let Some(v) = self.update_after {
            p.push(("updateAfter", v.unix_timestamp().to_string()));
        }
        Page::push(self.page, &mut p);
        p
    }
}

/// `GET /CreditNote`.
#[derive(Debug, Clone, Default)]
pub struct CreditNoteQuery {
    /// EXACT match.
    pub customer_internal_note: Option<String>,
    /// Compare exactly on the result: not measured to be an exact match.
    pub credit_note_number: Option<String>,
    pub page: Option<Page>,
}

impl CreditNoteQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let mut p = Pairs::new();
        if let Some(v) = &self.customer_internal_note {
            p.push(("customerInternalNote", v.clone()));
        }
        if let Some(v) = &self.credit_note_number {
            p.push(("creditNoteNumber", v.clone()));
        }
        Page::push(self.page, &mut p);
        p
    }
}
