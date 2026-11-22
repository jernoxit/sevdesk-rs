//! Check 7: the filters of the spec's list endpoints against the filters the
//! crate's query types send.
//!
//! Spec side, as counted in the write-type inventory: the `parameters` of the
//! `GET` operation plus the filter table of the tag description ("How to
//! filter ..."), `[objectName]` companions dropped, `tags[x][id]` for the tag
//! filter. Crate side: every query type built with EVERY field set, sent
//! against a mock server, and the query keys read off the captured request
//! (the query pairs are crate-private).
//!
//! The query literals below name every field without `..`: a new field of a
//! query type is a compile error here.

use super::exact_with_hint;
use serde_yaml_ng::Value;
use sevdesk::*;
use std::collections::BTreeSet;
use time::macros::datetime;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// `GET path:filter`: a filter of the spec that no query type sends, with the
/// reason (measured 2026-11-22, see the write-type protocol).
pub(super) const SPEC_FILTERS_NOT_IN_API: &[(&str, &str)] = &[
    (
        "GET /CreditNote:costCentre[id]",
        "not measurable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:creditNoteNumberNumberSmaller",
        "spec typo; ignored by sevDesk, creditNoteNumberSmaller works (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:customerIntenalNote",
        "spec typo; ignored by sevDesk, customerInternalNote works (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:day",
        "the spec form returns nothing (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:delinquent",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:headerOrNumber",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:headerStartsWith",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:notdelinquent",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:orderByDueDate",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:orderByDueTime",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /CreditNote:partiallyPaid",
        "not measurable: no partially paid credit note (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:costCentre[id]",
        "not measurable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:customerIntenalNote",
        "spec typo; ignored by sevDesk, customerInternalNote works (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:day",
        "the spec form returns nothing (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:headerOrNumber",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:headerStartsWith",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:notdelinquent",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:onlyDunned",
        "not measurable: no dunned invoice, needs a finalized document (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:orderByDueTime",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:showAll",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "GET /Invoice:showMa",
        "not measurable: needs a finalized document with a number (not done)",
    ),
    (
        "GET /Voucher:commentLike",
        "no effect, even with a whole position comment (measured 2026-11-22)",
    ),
    (
        "GET /Voucher:costCentre[id]",
        "not measurable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "GET /Voucher:fulltextSearchObjects",
        "a modifier of fulltextSearch without its own effect (measured 2026-11-22)",
    ),
    (
        "GET /Voucher:object[id]",
        "not measurable: no source for an id (measured 2026-11-22)",
    ),
    (
        "GET /Voucher:origin[id]",
        "not measurable: vouchers carry no origin (measured 2026-11-22)",
    ),
    (
        "GET /Voucher:withoutCatering",
        "unclear: no catering voucher in the data (measured 2026-11-22)",
    ),
];

/// `GET path:filter`: a key a query type sends that the spec does not list.
pub(super) const SENT_FILTERS_NOT_IN_SPEC: &[(&str, &str)] = &[
    (
        "GET /CheckAccountTransaction:limit",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /CheckAccountTransaction:offset",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /CheckAccountTransaction:status",
        "not in the spec; measured exact (2026-11-22)",
    ),
    (
        "GET /CreditNote:creditNoteNumberSmaller",
        "not in the spec (it names creditNoteNumberNumberSmaller); measured working (2026-11-22)",
    ),
    (
        "GET /CreditNote:customerInternalNote",
        "the spec's typo is customerIntenalNote; measured exact (2026-11-22)",
    ),
    (
        "GET /CreditNote:limit",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /CreditNote:offset",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /Invoice:customerInternalNote",
        "the spec's typo is customerIntenalNote; measured exact (2026-11-22)",
    ),
    (
        "GET /Invoice:limit",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /Invoice:offset",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /Voucher:limit",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /Voucher:offset",
        "paging; not in the spec for these operations, measured working (1..=1000)",
    ),
    (
        "GET /Voucher:supplier[id]",
        "not in the spec; measured exact, with a supplier on a voucher draft (2026-11-22)",
    ),
];

/// (path, tag name of the spec) of every list endpoint with a query type.
const ENDPOINTS: &[&str] = &[
    "/Invoice",
    "/CreditNote",
    "/Voucher",
    "/Contact",
    "/CheckAccountTransaction",
];

fn t(seconds: i64) -> Option<time::OffsetDateTime> {
    Some(datetime!(2026-11-22 12:00 UTC) + time::Duration::seconds(seconds))
}

fn amount() -> Option<Amount> {
    Some(Amount::from_minor(1750))
}

fn text() -> Option<String> {
    Some("x".into())
}

fn page() -> Option<Page> {
    Some(Page::new(5, 10).expect("valid page"))
}

fn invoice_query() -> InvoiceQuery {
    InvoiceQuery {
        customer_internal_note: text(),
        invoice_number: text(),
        status: Some(InvoiceStatus::Open),
        update_after: t(1),
        update_before: t(2),
        create_after: t(3),
        create_before: t(4),
        start_date: t(5),
        end_date: t(6),
        contact: Some(ContactId::new(1)),
        origin: Some(InvoiceId::new(1)),
        payment_method: Some(PaymentMethodId::new(1)),
        tag: Some(TagId::new(1)),
        invoice_type: Some(InvoiceType::Cancellation),
        send_type: Some(DocumentSendType::Vpdf),
        partially_paid: Some(true),
        delinquent: Some(true),
        show_wkr: Some(true),
        fulltext_search: text(),
        invoice_number_greater: text(),
        invoice_number_smaller: text(),
        order_by_invoice_number: Some(SortOrder::Asc),
        order_by_due_date: Some(true),
        order_by_debit: Some(true),
        page: page(),
    }
}

fn credit_note_query() -> CreditNoteQuery {
    CreditNoteQuery {
        customer_internal_note: text(),
        credit_note_number: text(),
        status: Some(CreditNoteStatus::Draft),
        start_date: t(1),
        end_date: t(2),
        contact: Some(ContactId::new(1)),
        origin: Some(InvoiceId::new(1)),
        payment_method: Some(PaymentMethodId::new(1)),
        tag: Some(TagId::new(1)),
        only_enshrined: Some(true),
        credit_note_number_greater: text(),
        credit_note_number_smaller: text(),
        start_amount: amount(),
        end_amount: amount(),
        order_by_credit_note_number: Some(SortOrder::Asc),
        order_by_debit: Some(true),
        page: page(),
    }
}

fn voucher_query() -> VoucherQuery {
    VoucherQuery {
        description_like: text(),
        status: Some(VoucherStatus::Draft),
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
        supplier_name: text(),
        search_comment_or_description: text(),
        fulltext_search: text(),
        accounting_type: Some(AccountingTypeId::new(1)),
        has_document: Some(true),
        start_amount: amount(),
        end_amount: amount(),
        contact: Some(ContactId::new(1)),
        contact_or_object: Some(ContactId::new(1)),
        supplier: Some(ContactId::new(1)),
        tag: Some(TagId::new(1)),
        delinquent: Some(true),
        no_rv: Some(true),
        order_by_voucher_number: Some(SortOrder::Asc),
        order_by_debit: Some(true),
        page: page(),
    }
}

fn contact_query() -> ContactQuery {
    ContactQuery {
        name: text(),
        customer_number: text(),
        category: Some(ContactCategoryId::new(1)),
        city: text(),
        zip: text(),
        country: Some(CountryId::new(1)),
        parent: Some(ContactId::new(1)),
        create_after: t(1),
        create_before: t(2),
        update_after: t(3),
        update_before: t(4),
        tag: Some(TagId::new(1)),
        depth: Some(ContactDepth::WithPersons),
        order_by_customer_number: Some(SortOrder::Asc),
    }
}

fn transaction_query() -> TransactionQuery {
    TransactionQuery {
        check_account: CheckAccountId::new(1),
        paymt_purpose: text(),
        start_date: t(1),
        end_date: t(2),
        is_booked: Some(true),
        status: Some(TransactionStatus::Created),
        payee_payer_name: text(),
        only_credit: Some(true),
        only_debit: Some(true),
        page: page(),
    }
}

/// A filter key as the spec writes it: no `[objectName]` companion, the tag
/// index as `x`.
fn normalize(key: &str) -> Option<String> {
    if key.ends_with("[objectName]") {
        return None;
    }
    let key = match key.strip_prefix("tags[") {
        Some(rest) if rest.ends_with("][id]") => "tags[x][id]".to_owned(),
        _ => key.to_owned(),
    };
    Some(key)
}

/// Sends every query type against a mock server; the keys of each request.
fn sent_filters() -> Vec<(&'static str, BTreeSet<String>)> {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"objects":[]}"#))
            .mount(&server)
            .await;
        let client = SevdeskClient::new(SevdeskClientConfig {
            base_url: server.uri(),
            api_token: ApiToken::new("spec-watch"),
            pacing: Pacing::NONE,
            x_version: None,
        })
        .expect("client");
        client
            .list_invoices(&invoice_query())
            .await
            .expect("invoices");
        client
            .list_credit_notes(&credit_note_query())
            .await
            .expect("credit notes");
        client
            .list_vouchers(&voucher_query())
            .await
            .expect("vouchers");
        client
            .list_contacts(&contact_query())
            .await
            .expect("contacts");
        client
            .list_transactions(&transaction_query())
            .await
            .expect("transactions");
        let requests = server.received_requests().await.expect("recorded requests");
        assert_eq!(requests.len(), ENDPOINTS.len(), "one request per endpoint");
        requests
            .iter()
            .zip(ENDPOINTS)
            .map(|(request, path)| {
                assert_eq!(request.url.path(), *path, "request order");
                let keys = request
                    .url
                    .query_pairs()
                    .filter_map(|(k, _)| normalize(&k))
                    .collect();
                (*path, keys)
            })
            .collect()
    })
}

/// The cells of the first column of every table row after "How to filter".
fn prose_filters(description: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let Some(start) = description.find("How to filter") else {
        return keys;
    };
    let mut rest = &description[start..];
    while let Some(row) = rest.find("<tr>") {
        rest = &rest[row + "<tr>".len()..];
        let Some(cell) = rest.trim_start().strip_prefix("<td>") else {
            continue;
        };
        let Some(end) = cell.find("</td>") else {
            break;
        };
        let mut plain = String::new();
        let mut in_tag = false;
        for c in cell[..end].chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => plain.push(c),
                _ => {}
            }
        }
        keys.extend(
            plain
                .split('&')
                .filter_map(|pair| pair.split('=').next())
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .filter_map(normalize),
        );
    }
    keys
}

/// The filters the spec names for `GET path`.
fn spec_filters(spec: &Value, path: &str) -> BTreeSet<String> {
    let operation = spec
        .get("paths")
        .and_then(|v| v.get(path))
        .and_then(|v| v.get("get"))
        .unwrap_or_else(|| panic!("spec has no GET {path}"));
    let mut keys: BTreeSet<String> = operation
        .get("parameters")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|p| p.get("name").and_then(Value::as_str))
        .filter_map(normalize)
        .collect();
    let tag = &path[1..];
    let description = spec
        .get("tags")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .find(|t| t.get("name").and_then(Value::as_str) == Some(tag))
        .and_then(|t| t.get("description"))
        .and_then(Value::as_str)
        .unwrap_or("");
    keys.extend(prose_filters(description));
    keys
}

pub(super) fn problems(spec: &Value) -> Vec<String> {
    let mut not_in_api = BTreeSet::new();
    let mut not_in_spec = BTreeSet::new();
    for (path, sent) in sent_filters() {
        let in_spec = spec_filters(spec, path);
        not_in_api.extend(in_spec.difference(&sent).map(|f| format!("GET {path}:{f}")));
        not_in_spec.extend(sent.difference(&in_spec).map(|f| format!("GET {path}:{f}")));
    }
    let mut out = Vec::new();
    out.extend(exact_with_hint(
        "SPEC_FILTERS_NOT_IN_API",
        &not_in_api,
        SPEC_FILTERS_NOT_IN_API,
        "a filter the crate does not send: a filter enters the API only after a read \
         measurement shows it narrows the result; measure it, then add it to the query \
         type and to the literal in filters.rs, or list it with the reason (ignored, \
         unclear, not measurable, spec typo)",
    ));
    out.extend(exact_with_hint(
        "SENT_FILTERS_NOT_IN_SPEC",
        &not_in_spec,
        SENT_FILTERS_NOT_IN_SPEC,
        "a key the crate sends that the spec lacks: list it with the measurement that \
         shows it works",
    ));
    out
}
