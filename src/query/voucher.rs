use super::*;
use crate::status::*;

// Measured 2026-11-22 and NOT offered: `commentLike` (no effect, even with a
// whole position comment), `withoutCatering` (unclear: no catering voucher in
// the data), `costCentre[id]` (not measurable), `object[id]`, `origin[id]`
// (no id source), `fulltextSearchObjects` (a modifier of `fulltextSearch`
// without its own effect), and the date text forms (only Unix seconds work).

/// `GET /Voucher`.
///
/// Every filter below is measured; sevDesk silently IGNORES a filter it does
/// not understand. Date bounds are Unix seconds; text filters ignore case.
#[derive(Debug, Clone, Default)]
pub struct VoucherQuery {
    /// SUBSTRING match at sevDesk: compare exactly on the result.
    pub description_like: Option<String>,
    /// EXACT match on the status code.
    pub status: Option<VoucherStatus>,
    /// `creditDebit`: exact (measured 2026-11-22).
    pub credit_debit: Option<CreditDebit>,
    /// `voucherType`: exact (measured 2026-11-22).
    pub voucher_type: Option<VoucherType>,
    /// `voucherDate >= T`, inclusive (`startDate`; measured 2026-11-22).
    pub start_date: Option<OffsetDateTime>,
    /// `voucherDate < T`, EXCLUSIVE (`endDate`), unlike the other end filters
    /// (measured 2026-11-22).
    pub end_date: Option<OffsetDateTime>,
    /// `create >= T`, inclusive (measured 2026-11-22).
    pub create_after: Option<OffsetDateTime>,
    /// `create <= T`, inclusive (measured 2026-11-22).
    pub create_before: Option<OffsetDateTime>,
    /// `update >= T`, inclusive (measured 2026-11-22).
    pub update_after: Option<OffsetDateTime>,
    /// `update <= T`, inclusive (measured 2026-11-22).
    pub update_before: Option<OffsetDateTime>,
    /// `payDate >= T`, inclusive; vouchers without a `payDate` drop out
    /// (`startPayDate`; measured 2026-11-22).
    pub start_pay_date: Option<OffsetDateTime>,
    /// `payDate <= T`, inclusive (`endPayDate`; measured 2026-11-22).
    pub end_pay_date: Option<OffsetDateTime>,
    /// `year`: four digits; the date field it cuts by is not determined
    /// (measured 2026-11-22).
    pub year: Option<u16>,
    /// `month` of `voucherDate` (measured 2026-11-22; completeness not checked
    /// against the set).
    pub month: Option<u8>,
    /// `supplierName`: SUBSTRING, ignoring case (measured 2026-11-22).
    pub supplier_name: Option<String>,
    /// `searchCommentOrDescription`: narrows the result; the matched fields
    /// are not determined (measured 2026-11-22).
    pub search_comment_or_description: Option<String>,
    /// `fulltextSearch`: narrows the result; the matched fields are not
    /// determined (measured 2026-11-22).
    pub fulltext_search: Option<String>,
    /// `accountingType[id]`: exact, over the `VoucherPos` (measured 2026-11-22).
    pub accounting_type: Option<AccountingTypeId>,
    /// `hasDocument=true`: exactly the vouchers with a document (measured
    /// 2026-11-22). Only `Some(true)` is sent.
    pub has_document: Option<bool>,
    /// `startAmount`: `sumGross >= B` or `sumNet >= B`, inclusive (the data
    /// cannot tell which); an exact decimal (measured 2026-11-22).
    pub start_amount: Option<Amount>,
    /// `endAmount`: `<= B`, inclusive; like [`Self::start_amount`].
    pub end_amount: Option<Amount>,
    /// `contact[id]`: the supplier, exact (measured 2026-11-22).
    pub contact: Option<ContactId>,
    /// `contactOrObject[id]`: exact, sent with `objectName=Contact`
    /// (measured 2026-11-22).
    pub contact_or_object: Option<ContactId>,
    /// `supplier[id]`: exact, sent with `objectName=Contact` (measured 2026-11-22).
    pub supplier: Option<ContactId>,
    /// One tag, sent as `tags[0][id]` (measured 2026-11-22).
    pub tag: Option<TagId>,
    /// `delinquent=true`: exactly the vouchers whose `paymentDeadline` lies in
    /// the past (measured 2026-11-22). Only `Some(true)` is sent.
    pub delinquent: Option<bool>,
    /// `noRv=true`: leaves out the recurring (`RV`) vouchers (measured
    /// 2026-11-22). Only `Some(true)` is sent.
    pub no_rv: Option<bool>,
    /// `orderByVoucherNumber`; `DESC` is the exact reverse of `ASC`; the
    /// order follows the last digits of `description` (measured 2026-11-22).
    pub order_by_voucher_number: Option<SortOrder>,
    /// `orderByDebit=true`: descending by `supplierName` as far as measured.
    /// Only `Some(true)` is sent (measured 2026-11-22).
    pub order_by_debit: Option<bool>,
    pub page: Option<Page>,
}

impl VoucherQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let Self {
            description_like,
            status,
            credit_debit,
            voucher_type,
            start_date,
            end_date,
            create_after,
            create_before,
            update_after,
            update_before,
            start_pay_date,
            end_pay_date,
            year,
            month,
            supplier_name,
            search_comment_or_description,
            fulltext_search,
            accounting_type,
            has_document,
            start_amount,
            end_amount,
            contact,
            contact_or_object,
            supplier,
            tag,
            delinquent,
            no_rv,
            order_by_voucher_number,
            order_by_debit,
            page,
        } = self;
        let mut p = Pairs::new();
        push_text(&mut p, "descriptionLike", description_like);
        if let Some(v) = status {
            p.push(("status", v.code().to_string()));
        }
        if let Some(v) = credit_debit {
            p.push(("creditDebit", v.as_str().to_owned()));
        }
        if let Some(v) = voucher_type {
            p.push(("voucherType", v.as_str().to_owned()));
        }
        push_time(&mut p, "startDate", *start_date);
        push_time(&mut p, "endDate", *end_date);
        push_time(&mut p, "createAfter", *create_after);
        push_time(&mut p, "createBefore", *create_before);
        push_time(&mut p, "updateAfter", *update_after);
        push_time(&mut p, "updateBefore", *update_before);
        push_time(&mut p, "startPayDate", *start_pay_date);
        push_time(&mut p, "endPayDate", *end_pay_date);
        if let Some(v) = year {
            p.push(("year", v.to_string()));
        }
        if let Some(v) = month {
            p.push(("month", v.to_string()));
        }
        push_text(&mut p, "supplierName", supplier_name);
        push_text(
            &mut p,
            "searchCommentOrDescription",
            search_comment_or_description,
        );
        push_text(&mut p, "fulltextSearch", fulltext_search);
        push_ref(
            &mut p,
            ("accountingType[id]", "accountingType[objectName]"),
            *accounting_type,
        );
        push_flag(&mut p, "hasDocument", *has_document);
        push_amount(&mut p, "startAmount", *start_amount);
        push_amount(&mut p, "endAmount", *end_amount);
        push_ref(&mut p, ("contact[id]", "contact[objectName]"), *contact);
        push_ref(
            &mut p,
            ("contactOrObject[id]", "contactOrObject[objectName]"),
            *contact_or_object,
        );
        push_ref(&mut p, ("supplier[id]", "supplier[objectName]"), *supplier);
        push_tag(&mut p, *tag);
        push_flag(&mut p, "delinquent", *delinquent);
        push_flag(&mut p, "noRv", *no_rv);
        push_order(&mut p, "orderByVoucherNumber", *order_by_voucher_number);
        push_flag(&mut p, "orderByDebit", *order_by_debit);
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
        let t = |s| Some(datetime!(2026-11-22 12:00 UTC) + time::Duration::seconds(s));
        let q = VoucherQuery {
            description_like: Some("desc".into()),
            status: Some(VoucherStatus::Open),
            credit_debit: Some(CreditDebit::Credit),
            voucher_type: Some(VoucherType::Normal),
            start_date: t(1),
            end_date: t(2),
            create_after: t(3),
            create_before: t(4),
            update_after: t(5),
            update_before: t(6),
            start_pay_date: t(7),
            end_pay_date: t(8),
            year: Some(2026),
            month: Some(11),
            supplier_name: Some("Acme".into()),
            search_comment_or_description: Some("sc".into()),
            fulltext_search: Some("ft".into()),
            accounting_type: Some(AccountingTypeId::new(20)),
            has_document: Some(true),
            start_amount: Some(Amount::from_minor(1000)),
            end_amount: Some(Amount::from_minor(2005)),
            contact: Some(ContactId::new(21)),
            contact_or_object: Some(ContactId::new(22)),
            supplier: Some(ContactId::new(23)),
            tag: Some(TagId::new(24)),
            delinquent: Some(true),
            no_rv: Some(true),
            order_by_voucher_number: Some(SortOrder::Asc),
            order_by_debit: Some(true),
            page: Some(Page::new(5, 10).unwrap()),
        };
        let base = 1_795_348_800;
        let expected: Vec<(&str, String)> = vec![
            ("descriptionLike", "desc".into()),
            ("status", "100".into()),
            ("creditDebit", "C".into()),
            ("voucherType", "VOU".into()),
            ("startDate", (base + 1).to_string()),
            ("endDate", (base + 2).to_string()),
            ("createAfter", (base + 3).to_string()),
            ("createBefore", (base + 4).to_string()),
            ("updateAfter", (base + 5).to_string()),
            ("updateBefore", (base + 6).to_string()),
            ("startPayDate", (base + 7).to_string()),
            ("endPayDate", (base + 8).to_string()),
            ("year", "2026".into()),
            ("month", "11".into()),
            ("supplierName", "Acme".into()),
            ("searchCommentOrDescription", "sc".into()),
            ("fulltextSearch", "ft".into()),
            ("accountingType[id]", "20".into()),
            ("accountingType[objectName]", "AccountingType".into()),
            ("hasDocument", "true".into()),
            ("startAmount", "10.00".into()),
            ("endAmount", "20.05".into()),
            ("contact[id]", "21".into()),
            ("contact[objectName]", "Contact".into()),
            ("contactOrObject[id]", "22".into()),
            ("contactOrObject[objectName]", "Contact".into()),
            ("supplier[id]", "23".into()),
            ("supplier[objectName]", "Contact".into()),
            ("tags[0][id]", "24".into()),
            ("tags[0][objectName]", "Tag".into()),
            ("delinquent", "true".into()),
            ("noRv", "true".into()),
            ("orderByVoucherNumber", "ASC".into()),
            ("orderByDebit", "true".into()),
            ("limit", "5".into()),
            ("offset", "10".into()),
        ];
        assert_eq!(q.pairs(), expected);
    }
}
