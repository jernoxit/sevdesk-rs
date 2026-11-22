use super::*;
use crate::status::*;

// Measured 2026-11-22 and NOT offered: the plain-date forms of `startDate` /
// `endDate` (`dd.mm.yyyy`, `yyyy-mm-dd`; Berlin day, `startDate` includes the
// day, `endDate` EXCLUDES it). They share the keys with the timestamp fields
// above, so offering both needs a rule for the clash, which the measurement
// does not give. RFC 3339 is not offered either: Unix seconds mean the same.

/// `GET /CheckAccountTransaction`. The list omits deleted transactions.
#[derive(Debug, Clone)]
pub struct TransactionQuery {
    pub check_account: CheckAccountId,
    /// SUBSTRING match at sevDesk, ignoring case: compare exactly on the result.
    pub paymt_purpose: Option<String>,
    /// Sent as a Unix timestamp; `entryDate >= T`, INCLUSIVE (measured 2026-11-22).
    /// Measured on the sevDesk test tenant (2026-10-01): the `startDate` and
    /// `endDate` filter cuts by `entryDate`, NOT by `valueDate`. Three
    /// transactions with the same `valueDate` and `entryDate`s of 01.10.,
    /// 10.10. and 20.10., queried with the window 05.-15.10., returned only
    /// the 10.10. one.
    pub start_date: Option<OffsetDateTime>,
    /// `entryDate <= T`, INCLUSIVE (measured 2026-11-22); cuts by `entryDate`,
    /// like [`Self::start_date`].
    pub end_date: Option<OffsetDateTime>,
    /// Measured: `true` returns only booked transactions (`status != 100`, so
    /// `200` and `400`), but `false` is IGNORED and returns everything. To ask
    /// for the open ones use [`Self::status`].
    pub is_booked: Option<bool>,
    /// Measured: `status=100` returns only transactions in that status.
    pub status: Option<TransactionStatus>,
    /// `payeePayerName`: SUBSTRING, ignoring case (measured 2026-11-22).
    pub payee_payer_name: Option<String>,
    /// `onlyCredit=true`: exactly the transactions with amount `< 0` (sic:
    /// the name is the other way round than it reads; measured 2026-11-22).
    /// Only `Some(true)` is sent.
    pub only_credit: Option<bool>,
    /// `onlyDebit=true`: exactly the transactions with amount `> 0` (measured
    /// 2026-11-22). Only `Some(true)` is sent.
    pub only_debit: Option<bool>,
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
            payee_payer_name: None,
            only_credit: None,
            only_debit: None,
            page: None,
        }
    }

    pub(crate) fn pairs(&self) -> Pairs {
        let Self {
            check_account,
            paymt_purpose,
            start_date,
            end_date,
            is_booked,
            status,
            payee_payer_name,
            only_credit,
            only_debit,
            page,
        } = self;
        let mut p: Pairs = vec![
            ("checkAccount[id]", check_account.to_string()),
            (
                "checkAccount[objectName]",
                CheckAccountId::OBJECT_NAME.into(),
            ),
        ];
        push_text(&mut p, "paymtPurpose", paymt_purpose);
        push_time(&mut p, "startDate", *start_date);
        push_time(&mut p, "endDate", *end_date);
        if let Some(v) = is_booked {
            p.push(("isBooked", v.to_string()));
        }
        if let Some(v) = status {
            p.push(("status", v.code().to_string()));
        }
        push_text(&mut p, "payeePayerName", payee_payer_name);
        push_flag(&mut p, "onlyCredit", *only_credit);
        push_flag(&mut p, "onlyDebit", *only_debit);
        Page::push(*page, &mut p);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn every_filter_is_encoded_as_measured() {
        let mut q = TransactionQuery::new(CheckAccountId::new(6));
        q.paymt_purpose = Some("purpose".into());
        q.start_date = Some(datetime!(2026-11-22 12:00 UTC));
        q.end_date = Some(datetime!(2026-11-22 12:00:05 UTC));
        q.is_booked = Some(true);
        q.status = Some(TransactionStatus::Created);
        q.payee_payer_name = Some("Payee".into());
        q.only_credit = Some(true);
        q.only_debit = Some(true);
        q.page = Some(Page::new(5, 10).unwrap());
        let expected: Vec<(&str, String)> = vec![
            ("checkAccount[id]", "6".into()),
            ("checkAccount[objectName]", "CheckAccount".into()),
            ("paymtPurpose", "purpose".into()),
            ("startDate", "1795348800".into()),
            ("endDate", "1795348805".into()),
            ("isBooked", "true".into()),
            ("status", "100".into()),
            ("payeePayerName", "Payee".into()),
            ("onlyCredit", "true".into()),
            ("onlyDebit", "true".into()),
            ("limit", "5".into()),
            ("offset", "10".into()),
        ];
        assert_eq!(q.pairs(), expected);
    }
}
