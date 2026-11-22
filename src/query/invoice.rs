use super::*;
use crate::status::*;

// Measured 2026-11-22 and NOT offered (sevDesk ignores them or they do not
// work as the spec says): `day` (spec form returns nothing), `showMa` and
// `onlyDunned` (not measured), `costCentre[id]` (not measurable), `showAll`,
// `notdelinquent`, `headerStartsWith`, `headerOrNumber`, `orderByDueTime`
// (all ignored), `customerIntenalNote` (the spec's typo, ignored), and the
// date text forms (only Unix seconds work).

/// `GET /Invoice`.
///
/// Every filter below is measured; sevDesk silently IGNORES a filter it does
/// not understand and answers with everything. Date bounds are Unix seconds,
/// compared as the real instant (no time zone shift); text filters ignore case.
#[derive(Debug, Clone, Default)]
pub struct InvoiceQuery {
    /// EXACT match; hits the original AND its cancellation invoice.
    pub customer_internal_note: Option<String>,
    /// EXACT match, ignoring case (measured 2026-11-22).
    pub invoice_number: Option<String>,
    /// EXACT match on the status code.
    pub status: Option<InvoiceStatus>,
    /// Changed at or after this instant (`update >=`, INCLUSIVE; measured
    /// 2026-11-22), sent as Unix seconds.
    /// [`crate::SevdeskClient::list_invoices`] checks every answer against it
    /// (a filter sevDesk does not understand is ignored and returns everything).
    pub update_after: Option<OffsetDateTime>,
    /// Changed at or before this instant (`update <=`, inclusive; measured 2026-11-22).
    pub update_before: Option<OffsetDateTime>,
    /// Created at or after this instant (`create >=`, inclusive; measured 2026-11-22).
    pub create_after: Option<OffsetDateTime>,
    /// Created at or before this instant (`create <=`, inclusive; measured 2026-11-22).
    pub create_before: Option<OffsetDateTime>,
    /// `invoiceDate >= T`, inclusive (`startDate`; measured 2026-11-22).
    pub start_date: Option<OffsetDateTime>,
    /// `invoiceDate <= T`, inclusive (`endDate`; measured 2026-11-22).
    pub end_date: Option<OffsetDateTime>,
    /// `contact[id]`: exact (measured 2026-11-22).
    pub contact: Option<ContactId>,
    /// `origin[id]`: exact, the original of a cancellation invoice
    /// (measured 2026-11-22).
    pub origin: Option<InvoiceId>,
    /// `paymentMethod[id]`: exact (measured 2026-11-22).
    pub payment_method: Option<PaymentMethodId>,
    /// One tag, sent as `tags[0][id]` (measured 2026-11-22).
    pub tag: Option<TagId>,
    /// `invoiceType`: exact, ignoring case (measured 2026-11-22).
    pub invoice_type: Option<InvoiceType>,
    /// `sendType`: exact (measured 2026-11-22).
    pub send_type: Option<DocumentSendType>,
    /// `partiallyPaid=true`: exactly the invoices with status `750`
    /// (measured 2026-11-22). Only `Some(true)` is sent.
    pub partially_paid: Option<bool>,
    /// `delinquent=true`: `invoiceDate + timeToPay days < now`, regardless of
    /// the status, so PAID invoices past that date count too (measured
    /// 2026-11-22). Only `Some(true)` is sent.
    pub delinquent: Option<bool>,
    /// `showWkr=true`: recurring invoice drafts (`WKR`) are hidden from the
    /// default list, and this returns ONLY them (measured 2026-11-22, with a
    /// recurring draft; unlike the spec). Only `Some(true)` is sent.
    pub show_wkr: Option<bool>,
    /// `fulltextSearch`: narrows the result; the matched fields are not
    /// determined (measured 2026-11-22).
    pub fulltext_search: Option<String>,
    /// `invoiceNumberGreater`: the WHOLE number as text, compared
    /// lexicographically and EXCLUSIVE (`>`); drafts without a number drop out.
    /// `RE-999` and `RE-1000` do not order numerically (measured 2026-11-22).
    pub invoice_number_greater: Option<String>,
    /// `invoiceNumberSmaller`: like [`Self::invoice_number_greater`], `<`.
    pub invoice_number_smaller: Option<String>,
    /// `orderByInvoiceNumber`, by the number as text; `DESC` is not the exact
    /// reverse of `ASC` (ties, drafts without a number). Measured 2026-11-22.
    pub order_by_invoice_number: Option<SortOrder>,
    /// `orderByDueDate=true`: ascending, by `invoiceDate` as far as measured;
    /// the key is not determined. Only `Some(true)` is sent (measured 2026-11-22).
    pub order_by_due_date: Option<bool>,
    /// `orderByDebit=true`: changes the order, the key is unknown. Only
    /// `Some(true)` is sent (measured 2026-11-22).
    pub order_by_debit: Option<bool>,
    pub page: Option<Page>,
}

impl InvoiceQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let Self {
            customer_internal_note,
            invoice_number,
            status,
            update_after,
            update_before,
            create_after,
            create_before,
            start_date,
            end_date,
            contact,
            origin,
            payment_method,
            tag,
            invoice_type,
            send_type,
            partially_paid,
            delinquent,
            show_wkr,
            fulltext_search,
            invoice_number_greater,
            invoice_number_smaller,
            order_by_invoice_number,
            order_by_due_date,
            order_by_debit,
            page,
        } = self;
        let mut p = Pairs::new();
        push_text(&mut p, "customerInternalNote", customer_internal_note);
        push_text(&mut p, "invoiceNumber", invoice_number);
        if let Some(v) = status {
            p.push(("status", v.code().to_string()));
        }
        push_time(&mut p, "updateAfter", *update_after);
        push_time(&mut p, "updateBefore", *update_before);
        push_time(&mut p, "createAfter", *create_after);
        push_time(&mut p, "createBefore", *create_before);
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
        if let Some(v) = invoice_type {
            p.push(("invoiceType", v.as_str().to_owned()));
        }
        if let Some(v) = send_type {
            p.push(("sendType", v.as_str().to_owned()));
        }
        push_flag(&mut p, "partiallyPaid", *partially_paid);
        push_flag(&mut p, "delinquent", *delinquent);
        push_flag(&mut p, "showWkr", *show_wkr);
        push_text(&mut p, "fulltextSearch", fulltext_search);
        push_text(&mut p, "invoiceNumberGreater", invoice_number_greater);
        push_text(&mut p, "invoiceNumberSmaller", invoice_number_smaller);
        push_order(&mut p, "orderByInvoiceNumber", *order_by_invoice_number);
        push_flag(&mut p, "orderByDueDate", *order_by_due_date);
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
        let q = InvoiceQuery {
            customer_internal_note: Some("note".into()),
            invoice_number: Some("RE-1".into()),
            status: Some(InvoiceStatus::Open),
            update_after: t(1),
            update_before: t(2),
            create_after: t(3),
            create_before: t(4),
            start_date: t(5),
            end_date: t(6),
            contact: Some(ContactId::new(7)),
            origin: Some(InvoiceId::new(8)),
            payment_method: Some(PaymentMethodId::new(9)),
            tag: Some(TagId::new(10)),
            invoice_type: Some(InvoiceType::Cancellation),
            send_type: Some(DocumentSendType::Vpdf),
            partially_paid: Some(true),
            delinquent: Some(true),
            show_wkr: Some(true),
            fulltext_search: Some("text".into()),
            invoice_number_greater: Some("RE-2".into()),
            invoice_number_smaller: Some("RE-3".into()),
            order_by_invoice_number: Some(SortOrder::Desc),
            order_by_due_date: Some(true),
            order_by_debit: Some(true),
            page: Some(Page::new(5, 10).unwrap()),
        };
        let base = 1_795_348_800; // 2026-11-22 12:00 UTC
        let expected: Vec<(&str, String)> = vec![
            ("customerInternalNote", "note".into()),
            ("invoiceNumber", "RE-1".into()),
            ("status", "200".into()),
            ("updateAfter", (base + 1).to_string()),
            ("updateBefore", (base + 2).to_string()),
            ("createAfter", (base + 3).to_string()),
            ("createBefore", (base + 4).to_string()),
            ("startDate", (base + 5).to_string()),
            ("endDate", (base + 6).to_string()),
            ("contact[id]", "7".into()),
            ("contact[objectName]", "Contact".into()),
            ("origin[id]", "8".into()),
            ("origin[objectName]", "Invoice".into()),
            ("paymentMethod[id]", "9".into()),
            ("paymentMethod[objectName]", "PaymentMethod".into()),
            ("tags[0][id]", "10".into()),
            ("tags[0][objectName]", "Tag".into()),
            ("invoiceType", "SR".into()),
            ("sendType", "VPDF".into()),
            ("partiallyPaid", "true".into()),
            ("delinquent", "true".into()),
            ("showWkr", "true".into()),
            ("fulltextSearch", "text".into()),
            ("invoiceNumberGreater", "RE-2".into()),
            ("invoiceNumberSmaller", "RE-3".into()),
            ("orderByInvoiceNumber", "DESC".into()),
            ("orderByDueDate", "true".into()),
            ("orderByDebit", "true".into()),
            ("limit", "5".into()),
            ("offset", "10".into()),
        ];
        assert_eq!(q.pairs(), expected);
    }
}
