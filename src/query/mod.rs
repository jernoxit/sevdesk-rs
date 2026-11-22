//! Query parameters of the list endpoints, as typed values.
//!
//! sevDesk's filters are name-sensitive and fail silently: a misspelled or
//! unknown filter is IGNORED and the call answers with everything (measured
//! for `customerIntenalNote`, the spec's typo, and for `externalId`). Every
//! key here is a measured one.

use crate::Amount;
use crate::codec::format_decimal;
use crate::error::SevdeskError;
use crate::ids::*;
use time::OffsetDateTime;

mod contact;
mod credit_note;
mod invoice;
mod transaction;
mod voucher;

pub use contact::{ContactDepth, ContactQuery};
pub use credit_note::CreditNoteQuery;
pub use invoice::InvoiceQuery;
pub use transaction::TransactionQuery;
pub use voucher::VoucherQuery;

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

    pub(crate) fn push(page: Option<Self>, pairs: &mut Pairs) {
        if let Some(Self { limit, offset }) = page {
            pairs.push(("limit", limit.to_string()));
            pairs.push(("offset", offset.to_string()));
        }
    }
}

/// Sort direction of an `orderBy…Number` parameter; sent as `ASC` / `DESC`
/// (measured 2026-11-22). The sort changes the order, not the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

pub(crate) fn push_text(p: &mut Pairs, key: &'static str, v: &Option<String>) {
    if let Some(v) = v {
        p.push((key, v.clone()));
    }
}

/// A Unix-seconds date filter (the only form the list filters understand).
pub(crate) fn push_time(p: &mut Pairs, key: &'static str, v: Option<OffsetDateTime>) {
    if let Some(v) = v {
        p.push((key, v.unix_timestamp().to_string()));
    }
}

/// A `true`-only flag: measured with `true` only, so `Some(false)` sends nothing.
pub(crate) fn push_flag(p: &mut Pairs, key: &'static str, v: Option<bool>) {
    if v == Some(true) {
        p.push((key, "true".into()));
    }
}

pub(crate) fn push_amount(p: &mut Pairs, key: &'static str, v: Option<Amount>) {
    if let Some(v) = v {
        p.push((key, format_decimal(v)));
    }
}

pub(crate) fn push_order(p: &mut Pairs, key: &'static str, v: Option<SortOrder>) {
    if let Some(v) = v {
        p.push((key, v.as_str().into()));
    }
}

/// `x[id]=…&x[objectName]=…`; the caller passes both keys.
pub(crate) fn push_ref<I: Resource + std::fmt::Display>(
    p: &mut Pairs,
    keys: (&'static str, &'static str),
    v: Option<I>,
) {
    if let Some(v) = v {
        p.push((keys.0, v.to_string()));
        p.push((keys.1, I::OBJECT_NAME.into()));
    }
}

/// `tags[0][id]=…&tags[0][objectName]=Tag`; measured 2026-11-22: the form
/// without an index (`tags[id]`) is an error.
pub(crate) fn push_tag(p: &mut Pairs, v: Option<TagId>) {
    push_ref(p, ("tags[0][id]", "tags[0][objectName]"), v);
}
