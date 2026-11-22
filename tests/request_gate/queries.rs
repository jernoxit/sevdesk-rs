//! Part 2, read-only: every query parameter that commit `e104b68` added, sent
//! once through the crate's list method with a value taken from live data (the
//! objects of part 1 as read back, or existing objects found read-only).

use crate::gate::{Expect, id_of, instant_of};
use crate::harness::{BANK, account, eur};
use crate::{Facts, Gate};
use reqwest::Method;
use sevdesk::*;
use time::Duration;
use time::macros::datetime;

use Expect::{Narrows, NonEmpty, Same, Wider};

fn page() -> Option<Page> {
    Some(Page::new(1000, 0).unwrap())
}

/// Sends one query and records the class.
macro_rules! q {
    ($g:expr, $ctx:expr, $name:expr, $param:literal, $expect:expr, $base:expr, $call:expr) => {{
        let _ = $ctx;
        let result = $call.await.map(|v| v.len());
        $g.rec.query($name, $param, $expect, $base, result, "");
    }};
}

/// The first tag attached to an object of `kind` (read-only, `TagRelation`).
async fn tag_for(g: &Gate<'_>, kind: &str) -> Option<TagId> {
    let (status, text) = g
        .ctx
        .raw(Method::GET, "/TagRelation?limit=1000", None, None)
        .await;
    let tag = (status < 300)
        .then(|| serde_json::from_str::<serde_json::Value>(&text).ok())
        .flatten()
        .and_then(|v| v["objects"].as_array().cloned())
        .and_then(|list| {
            list.iter()
                .find(|r| r["object"]["objectName"] == kind)
                .and_then(|r| id_of(&r["tag"]))
        });
    eprintln!("LOOKUP\tTagRelation for {kind}\tHTTP {status}\t{tag:?}");
    tag.map(TagId::new)
}

fn text(v: &serde_json::Value) -> String {
    v.as_str().unwrap_or_default().to_owned()
}

pub(crate) async fn run(g: &mut Gate<'_>, f: &Facts) {
    invoices(g, f).await;
    credit_notes(g, f).await;
    vouchers(g, f).await;
    transactions(g, f).await;
    contacts(g, f).await;
}

async fn invoices(g: &mut Gate<'_>, f: &Facts) {
    const N: &str = "InvoiceQuery";
    let ctx = g.ctx;
    let all = match ctx
        .c
        .list_invoices(&InvoiceQuery {
            page: page(),
            ..Default::default()
        })
        .await
    {
        Ok(all) => all,
        Err(e) => {
            g.rec
                .query(N, "(unfiltered)", NonEmpty, 0, Err::<usize, _>(e), "");
            return;
        }
    };
    let u = all.len();
    let origin = g.inp.origin.clone();
    let created = instant_of(&f.invoice["create"]);
    let contact =
        ContactId::new(id_of(&f.invoice["contact"]).expect("the invoice names its contact"));
    let pm = g.inp.pm;
    let marker = ctx.marker.clone();
    let origin_number = origin.invoice_number.clone().unwrap_or_default();
    let of_cancellation = all
        .iter()
        .find(|i| i.invoice_type == Some(InvoiceType::Cancellation))
        .and_then(|i| i.origin.map(|o| o.id));
    let tag = tag_for(g, "Invoice").await;
    let p = page();

    q!(
        g,
        ctx,
        N,
        "update_before",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            update_before: origin.update,
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "create_after",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            create_after: Some(created),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "create_before",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            create_before: origin.create,
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "start_date",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            start_date: Some(datetime!(2026-10-03 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "end_date",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            end_date: Some(datetime!(2026-10-02 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "contact",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            contact: Some(contact),
            page: p,
            ..Default::default()
        })
    );
    match of_cancellation {
        Some(original) => q!(
            g,
            ctx,
            N,
            "origin",
            Narrows,
            u,
            ctx.c.list_invoices(&InvoiceQuery {
                origin: Some(original),
                page: p,
                ..Default::default()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "origin", u, "no cancellation invoice in the account"),
    }
    q!(
        g,
        ctx,
        N,
        "payment_method",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            payment_method: Some(pm),
            page: p,
            ..Default::default()
        })
    );
    match tag {
        Some(tag) => q!(
            g,
            ctx,
            N,
            "tag",
            Narrows,
            u,
            ctx.c.list_invoices(&InvoiceQuery {
                tag: Some(tag),
                page: p,
                ..Default::default()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "tag", u, "no tag relation to an invoice"),
    }
    q!(
        g,
        ctx,
        N,
        "invoice_type",
        NonEmpty,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            invoice_type: Some(InvoiceType::Invoice),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "send_type",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            send_type: Some(DocumentSendType::Vpdf),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "partially_paid",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            partially_paid: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "delinquent",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            delinquent: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "show_wkr",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            show_wkr: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "fulltext_search",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            fulltext_search: Some(marker.clone()),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "invoice_number_greater",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            invoice_number_greater: Some(origin_number),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "invoice_number_smaller",
        Narrows,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            invoice_number_smaller: Some(f.invoice_number.clone()),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_invoice_number",
        Same,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            order_by_invoice_number: Some(SortOrder::Asc),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_due_date",
        Same,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            order_by_due_date: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_debit",
        Same,
        u,
        ctx.c.list_invoices(&InvoiceQuery {
            order_by_debit: Some(true),
            page: p,
            ..Default::default()
        })
    );
}

async fn credit_notes(g: &mut Gate<'_>, f: &Facts) {
    const N: &str = "CreditNoteQuery";
    let ctx = g.ctx;
    let p = page();
    let all = match ctx
        .c
        .list_credit_notes(&CreditNoteQuery {
            page: p,
            ..Default::default()
        })
        .await
    {
        Ok(all) => all,
        Err(e) => {
            g.rec
                .query(N, "(unfiltered)", NonEmpty, 0, Err::<usize, _>(e), "");
            return;
        }
    };
    let u = all.len();
    let cn = &f.credit_note;
    let contact = ContactId::new(id_of(&cn["contact"]).expect("the credit note names its contact"));
    let origin = g.inp.origin.id;
    let pm = g.inp.pm;
    let own_number = text(&cn["creditNoteNumber"]);
    let lowest_other = all
        .iter()
        .filter_map(|c| c.credit_note_number.clone())
        .filter(|n| *n != own_number)
        .min();
    let tag = tag_for(g, "CreditNote").await;

    q!(
        g,
        ctx,
        N,
        "status",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            status: Some(CreditNoteStatus::Draft),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "start_date",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            start_date: Some(datetime!(2026-10-04 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "end_date",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            end_date: Some(datetime!(2026-10-03 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "contact",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            contact: Some(contact),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "origin",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            origin: Some(origin),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "payment_method",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            payment_method: Some(pm),
            page: p,
            ..Default::default()
        })
    );
    match tag {
        Some(tag) => q!(
            g,
            ctx,
            N,
            "tag",
            Narrows,
            u,
            ctx.c.list_credit_notes(&CreditNoteQuery {
                tag: Some(tag),
                page: p,
                ..Default::default()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "tag", u, "no tag relation to a credit note"),
    }
    q!(
        g,
        ctx,
        N,
        "only_enshrined",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            only_enshrined: Some(true),
            page: p,
            ..Default::default()
        })
    );
    match lowest_other {
        Some(lowest) => q!(
            g,
            ctx,
            N,
            "credit_note_number_greater",
            Narrows,
            u,
            ctx.c.list_credit_notes(&CreditNoteQuery {
                credit_note_number_greater: Some(lowest),
                page: p,
                ..Default::default()
            })
        ),
        None => g.rec.query_skipped(
            N,
            "credit_note_number_greater",
            u,
            "no other credit note with a number",
        ),
    }
    q!(
        g,
        ctx,
        N,
        "credit_note_number_smaller",
        Narrows,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            credit_note_number_smaller: Some(own_number),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "start_amount",
        NonEmpty,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            start_amount: Some(eur(crate::setup::CREDIT_NET)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "end_amount",
        NonEmpty,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            end_amount: Some(eur(crate::setup::CREDIT_NET)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_credit_note_number",
        Same,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            order_by_credit_note_number: Some(SortOrder::Asc),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_debit",
        Same,
        u,
        ctx.c.list_credit_notes(&CreditNoteQuery {
            order_by_debit: Some(true),
            page: p,
            ..Default::default()
        })
    );
}

async fn vouchers(g: &mut Gate<'_>, f: &Facts) {
    const N: &str = "VoucherQuery";
    let ctx = g.ctx;
    let p = page();
    let u = match ctx
        .c
        .list_vouchers(&VoucherQuery {
            page: p,
            ..Default::default()
        })
        .await
    {
        Ok(all) => all.len(),
        Err(e) => {
            g.rec
                .query(N, "(unfiltered)", NonEmpty, 0, Err::<usize, _>(e), "");
            return;
        }
    };
    let v = &f.voucher;
    let created = instant_of(&v["create"]);
    let updated = instant_of(&v["update"]);
    let supplier = ContactId::new(id_of(&v["supplier"]).expect("the voucher names its supplier"));
    let supplier_name = text(&v["supplierName"]);
    let description = text(&v["description"]);
    let marker = ctx.marker.clone();
    let accounting_type = id_of(&f.voucher_pos["accountingType"])
        .map(AccountingTypeId::new)
        .or(g.inp.accounting_type);
    let has_document = v.get("document").is_some_and(|d| !d.is_null());
    let sum = |k: &str| {
        v[k].as_str()
            .and_then(|s| parse_amount(s).ok())
            .unwrap_or_else(|| eur(1600))
    };
    let (gross, net) = (sum("sumGross"), sum("sumNet"));
    let tag = tag_for(g, "Voucher").await;
    let minute = Duration::seconds(60);

    q!(
        g,
        ctx,
        N,
        "credit_debit",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            credit_debit: Some(CreditDebit::Credit),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "voucher_type",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            voucher_type: Some(VoucherType::Normal),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "start_date",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            start_date: Some(datetime!(2026-10-02 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "end_date",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            end_date: Some(datetime!(2026-10-02 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "create_after",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            create_after: Some(created),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "create_before",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            create_before: Some(created - minute),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "update_after",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            update_after: Some(updated),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "update_before",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            update_before: Some(updated - minute),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "start_pay_date",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            start_pay_date: Some(datetime!(2026-10-11 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "end_pay_date",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            end_pay_date: Some(datetime!(2026-10-12 0:00 +2)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "year",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            year: Some(2026),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "month",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            month: Some(10),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "supplier_name",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            supplier_name: Some(supplier_name),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "search_comment_or_description",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            search_comment_or_description: Some(description),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "fulltext_search",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            fulltext_search: Some(marker),
            page: p,
            ..Default::default()
        })
    );
    match accounting_type {
        Some(at) => q!(
            g,
            ctx,
            N,
            "accounting_type",
            NonEmpty,
            u,
            ctx.c.list_vouchers(&VoucherQuery {
                accounting_type: Some(at),
                page: p,
                ..Default::default()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "accounting_type", u, "no accounting type id"),
    }
    let doc_expect = if has_document { Narrows } else { NonEmpty };
    q!(
        g,
        ctx,
        N,
        "has_document",
        doc_expect,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            has_document: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "start_amount",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            start_amount: Some(gross),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "end_amount",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            end_amount: Some(net.max(gross)),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "contact",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            contact: Some(supplier),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "contact_or_object",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            contact_or_object: Some(supplier),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "supplier",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            supplier: Some(supplier),
            page: p,
            ..Default::default()
        })
    );
    match tag {
        Some(tag) => q!(
            g,
            ctx,
            N,
            "tag",
            Narrows,
            u,
            ctx.c.list_vouchers(&VoucherQuery {
                tag: Some(tag),
                page: p,
                ..Default::default()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "tag", u, "no tag relation to a voucher"),
    }
    q!(
        g,
        ctx,
        N,
        "delinquent",
        Narrows,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            delinquent: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "no_rv",
        NonEmpty,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            no_rv: Some(true),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_voucher_number",
        Same,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            order_by_voucher_number: Some(SortOrder::Asc),
            page: p,
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_debit",
        Same,
        u,
        ctx.c.list_vouchers(&VoucherQuery {
            order_by_debit: Some(true),
            page: p,
            ..Default::default()
        })
    );
}

async fn transactions(g: &mut Gate<'_>, f: &Facts) {
    const N: &str = "TransactionQuery";
    let ctx = g.ctx;
    let p = page();
    let on = account(id_of(&f.transaction["checkAccount"]).unwrap_or(BANK));
    let base = || TransactionQuery {
        page: p,
        ..TransactionQuery::new(on)
    };
    let u = match ctx.c.list_transactions(&base()).await {
        Ok(all) => all.len(),
        Err(e) => {
            g.rec
                .query(N, "(unfiltered)", NonEmpty, 0, Err::<usize, _>(e), "");
            return;
        }
    };
    let payee = text(&f.transaction["payeePayerName"]);
    q!(
        g,
        ctx,
        N,
        "payee_payer_name",
        Narrows,
        u,
        ctx.c.list_transactions(&TransactionQuery {
            payee_payer_name: Some(payee),
            ..base()
        })
    );
    q!(
        g,
        ctx,
        N,
        "only_credit",
        Narrows,
        u,
        ctx.c.list_transactions(&TransactionQuery {
            only_credit: Some(true),
            ..base()
        })
    );
    q!(
        g,
        ctx,
        N,
        "only_debit",
        NonEmpty,
        u,
        ctx.c.list_transactions(&TransactionQuery {
            only_debit: Some(true),
            ..base()
        })
    );
}

async fn contacts(g: &mut Gate<'_>, f: &Facts) {
    const N: &str = "ContactQuery";
    let ctx = g.ctx;
    let persons = ContactQuery {
        depth: Some(ContactDepth::WithPersons),
        ..Default::default()
    };
    // Our contact is a person: only `depth=1` lists it, so its queries run on
    // the widened list (the `depth` row itself is judged against the default).
    let (u0, u1) = match (
        ctx.c.list_contacts(&ContactQuery::default()).await,
        ctx.c.list_contacts(&persons).await,
    ) {
        (Ok(a), Ok(b)) => (a.len(), b.len()),
        (a, b) => {
            let e = a.err().or(b.err()).expect("one of the two failed");
            g.rec
                .query(N, "(unfiltered)", NonEmpty, 0, Err::<usize, _>(e), "");
            return;
        }
    };
    let c = &f.contact;
    let created = instant_of(&c["create"]);
    let updated = instant_of(&c["update"]);
    let wide = || ContactQuery {
        depth: Some(ContactDepth::WithPersons),
        ..Default::default()
    };
    let tag = tag_for(g, "Contact").await;
    let minute = Duration::seconds(60);
    let (name, number) = (text(&c["name"]), text(&c["customerNumber"]));
    let (city, zip) = (text(&f.address["city"]), text(&f.address["zip"]));
    let category = id_of(&c["category"]).map(ContactCategoryId::new);
    let country = id_of(&f.address["country"]).map(CountryId::new);
    let parent = id_of(&c["parent"]).map(ContactId::new);

    q!(
        g,
        ctx,
        N,
        "name",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            name: Some(name),
            ..wide()
        })
    );
    q!(
        g,
        ctx,
        N,
        "customer_number",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            customer_number: Some(number),
            ..wide()
        })
    );
    match category {
        Some(v) => q!(
            g,
            ctx,
            N,
            "category",
            NonEmpty,
            u1,
            ctx.c.list_contacts(&ContactQuery {
                category: Some(v),
                ..wide()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "category", u1, "the contact reads back no category"),
    }
    q!(
        g,
        ctx,
        N,
        "city",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            city: Some(city),
            ..wide()
        })
    );
    q!(
        g,
        ctx,
        N,
        "zip",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            zip: Some(zip),
            ..wide()
        })
    );
    match country {
        Some(v) => q!(
            g,
            ctx,
            N,
            "country",
            NonEmpty,
            u1,
            ctx.c.list_contacts(&ContactQuery {
                country: Some(v),
                ..wide()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "country", u1, "the address reads back no country"),
    }
    match parent {
        Some(v) => q!(
            g,
            ctx,
            N,
            "parent",
            Narrows,
            u1,
            ctx.c.list_contacts(&ContactQuery {
                parent: Some(v),
                ..wide()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "parent", u1, "the contact reads back no parent"),
    }
    q!(
        g,
        ctx,
        N,
        "create_after",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            create_after: Some(created),
            ..wide()
        })
    );
    q!(
        g,
        ctx,
        N,
        "create_before",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            create_before: Some(created - minute),
            ..wide()
        })
    );
    q!(
        g,
        ctx,
        N,
        "update_after",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            update_after: Some(updated),
            ..wide()
        })
    );
    q!(
        g,
        ctx,
        N,
        "update_before",
        Narrows,
        u1,
        ctx.c.list_contacts(&ContactQuery {
            update_before: Some(updated - minute),
            ..wide()
        })
    );
    match tag {
        Some(tag) => q!(
            g,
            ctx,
            N,
            "tag",
            Narrows,
            u0,
            ctx.c.list_contacts(&ContactQuery {
                tag: Some(tag),
                ..Default::default()
            })
        ),
        None => g
            .rec
            .query_skipped(N, "tag", u0, "no tag relation to a contact"),
    }
    q!(
        g,
        ctx,
        N,
        "depth",
        Wider,
        u0,
        ctx.c.list_contacts(&ContactQuery {
            depth: Some(ContactDepth::WithPersons),
            ..Default::default()
        })
    );
    q!(
        g,
        ctx,
        N,
        "order_by_customer_number",
        Same,
        u0,
        ctx.c.list_contacts(&ContactQuery {
            order_by_customer_number: Some(SortOrder::Asc),
            ..Default::default()
        })
    );
}
