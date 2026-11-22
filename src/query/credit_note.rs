use super::*;
use crate::status::*;

// Measured 2026-11-22 and NOT offered: `day` (spec form returns nothing),
// `partiallyPaid` (not measured: no part-paid credit note), `costCentre[id]`
// (not measurable), `delinquent`, `notdelinquent`, `headerOrNumber`,
// `headerStartsWith`, `orderByDueDate`, `orderByDueTime` (all ignored),
// `creditNoteNumberNumberSmaller` (the spec's typo, ignored; the working key
// is `creditNoteNumberSmaller`), `customerIntenalNote` (spec typo, ignored),
// and the date text forms (only Unix seconds work).

/// `GET /CreditNote`.
///
/// Every filter below is measured; sevDesk silently IGNORES a filter it does
/// not understand. Date bounds are Unix seconds; text filters ignore case.
#[derive(Debug, Clone, Default)]
pub struct CreditNoteQuery {
    /// EXACT match.
    pub customer_internal_note: Option<String>,
    /// EXACT match (measured 2026-11-22: a number minus its last character finds nothing).
    pub credit_note_number: Option<String>,
    /// EXACT match on the status code (measured 2026-11-22).
    pub status: Option<CreditNoteStatus>,
    /// `creditNoteDate >= T`, inclusive (`startDate`; measured 2026-11-22).
    pub start_date: Option<OffsetDateTime>,
    /// `creditNoteDate <= T`, inclusive (`endDate`; measured 2026-11-22).
    pub end_date: Option<OffsetDateTime>,
    /// `contact[id]`: exact (measured 2026-11-22).
    pub contact: Option<ContactId>,
    /// `origin[id]`: exact; the origin is an invoice (measured 2026-11-22).
    pub origin: Option<InvoiceId>,
    /// `paymentMethod[id]`: exact (measured 2026-11-22).
    pub payment_method: Option<PaymentMethodId>,
    /// One tag, sent as `tags[0][id]` (measured 2026-11-22).
    pub tag: Option<TagId>,
    /// `onlyEnshrined=true`: exactly the enshrined credit notes (measured
    /// 2026-11-22). Only `Some(true)` is sent.
    pub only_enshrined: Option<bool>,
    /// `creditNoteNumberGreater`: the WHOLE number as text, lexicographic and
    /// EXCLUSIVE (`>`); drafts without a number drop out. Numbers of different
    /// length do not order numerically (measured 2026-11-22).
    pub credit_note_number_greater: Option<String>,
    /// `creditNoteNumberSmaller`: like [`Self::credit_note_number_greater`], `<`.
    pub credit_note_number_smaller: Option<String>,
    /// `startAmount`: `sumNet >= B`, inclusive; sent as an exact decimal
    /// (`sumGross` does not match; measured 2026-11-22).
    pub start_amount: Option<Amount>,
    /// `endAmount`: `sumNet <= B`, inclusive (measured 2026-11-22).
    pub end_amount: Option<Amount>,
    /// `orderByCreditNoteNumber`; `DESC` is not the exact reverse of `ASC`
    /// (ties). Measured 2026-11-22.
    pub order_by_credit_note_number: Option<SortOrder>,
    /// `orderByDebit=true`: changes the order, the key is unknown. Only
    /// `Some(true)` is sent (measured 2026-11-22).
    pub order_by_debit: Option<bool>,
    pub page: Option<Page>,
}

impl CreditNoteQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let Self {
            customer_internal_note,
            credit_note_number,
            status,
            start_date,
            end_date,
            contact,
            origin,
            payment_method,
            tag,
            only_enshrined,
            credit_note_number_greater,
            credit_note_number_smaller,
            start_amount,
            end_amount,
            order_by_credit_note_number,
            order_by_debit,
            page,
        } = self;
        let mut p = Pairs::new();
        push_text(&mut p, "customerInternalNote", customer_internal_note);
        push_text(&mut p, "creditNoteNumber", credit_note_number);
        if let Some(v) = status {
            p.push(("status", v.code().to_string()));
        }
        push_time(&mut p, "startDate", *start_date);
        push_time(&mut p, "endDate", *end_date);
        push_ref(&mut p, ("contact[id]", "contact[objectName]"), *contact);
        push_ref(&mut p, ("origin[id]", "origin[objectName]"), *origin);
        push_ref(
            &mut p,
            ("paymentMethod[id]", "paymentMethod[objectName]"),
            *payment_method,
        );
        push_tag(&mut p, *tag);
        push_flag(&mut p, "onlyEnshrined", *only_enshrined);
        push_text(
            &mut p,
            "creditNoteNumberGreater",
            credit_note_number_greater,
        );
        push_text(
            &mut p,
            "creditNoteNumberSmaller",
            credit_note_number_smaller,
        );
        push_amount(&mut p, "startAmount", *start_amount);
        push_amount(&mut p, "endAmount", *end_amount);
        push_order(
            &mut p,
            "orderByCreditNoteNumber",
            *order_by_credit_note_number,
        );
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
        let q = CreditNoteQuery {
            customer_internal_note: Some("note".into()),
            credit_note_number: Some("GU-1".into()),
            status: Some(CreditNoteStatus::Open),
            start_date: Some(datetime!(2026-11-22 12:00 UTC)),
            end_date: Some(datetime!(2026-11-22 12:00:05 UTC)),
            contact: Some(ContactId::new(7)),
            origin: Some(InvoiceId::new(8)),
            payment_method: Some(PaymentMethodId::new(9)),
            tag: Some(TagId::new(10)),
            only_enshrined: Some(true),
            credit_note_number_greater: Some("GU-2".into()),
            credit_note_number_smaller: Some("GU-3".into()),
            start_amount: Some(Amount::from_minor(1750)),
            end_amount: Some(Amount::from_minor(-5)),
            order_by_credit_note_number: Some(SortOrder::Asc),
            order_by_debit: Some(true),
            page: Some(Page::new(5, 10).unwrap()),
        };
        let expected: Vec<(&str, String)> = vec![
            ("customerInternalNote", "note".into()),
            ("creditNoteNumber", "GU-1".into()),
            ("status", "200".into()),
            ("startDate", "1795348800".into()),
            ("endDate", "1795348805".into()),
            ("contact[id]", "7".into()),
            ("contact[objectName]", "Contact".into()),
            ("origin[id]", "8".into()),
            ("origin[objectName]", "Invoice".into()),
            ("paymentMethod[id]", "9".into()),
            ("paymentMethod[objectName]", "PaymentMethod".into()),
            ("tags[0][id]", "10".into()),
            ("tags[0][objectName]", "Tag".into()),
            ("onlyEnshrined", "true".into()),
            ("creditNoteNumberGreater", "GU-2".into()),
            ("creditNoteNumberSmaller", "GU-3".into()),
            ("startAmount", "17.50".into()),
            ("endAmount", "-0.05".into()),
            ("orderByCreditNoteNumber", "ASC".into()),
            ("orderByDebit", "true".into()),
            ("limit", "5".into()),
            ("offset", "10".into()),
        ];
        assert_eq!(q.pairs(), expected);
    }
}
