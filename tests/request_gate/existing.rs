//! Inputs of the read-only query gate (`case2_queries_read_only`): the `Facts`
//! that part 1 reads back after its writes are taken from EXISTING objects
//! instead, newest first, preferring those that carry an `SDRS-GATE-` marker.
//! Every request here is a `GET`.

use crate::docs::raw_objects;
use crate::gate::{Expect, id_of};
use crate::harness::{BANK, Ctx};
use crate::{Facts, Gate};
use reqwest::Method;
use serde_json::Value;
use sevdesk::{ContactQuery, Page};

const MARKER_PREFIX: &str = "SDRS-GATE-";

/// The only method this module ever sends.
fn only_get(method: Method) -> Method {
    assert_eq!(method, Method::GET, "the read-only gate sends GET only");
    method
}

/// `GET <path>` as a list of raw objects; a failing read is no measurement.
pub(crate) async fn get_all(ctx: &Ctx, path: &str) -> Vec<Value> {
    let (status, text) = ctx.raw(only_get(Method::GET), path, None, None).await;
    assert!(status < 300, "GET {path} answered {status}");
    match raw_objects(&text) {
        Value::Array(list) => list,
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

fn id(v: &Value) -> i64 {
    id_of(v).unwrap_or(0)
}

fn carries_marker(v: &Value, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|f| v[*f].as_str().is_some_and(|s| s.contains(MARKER_PREFIX)))
}

/// The newest (highest id) object of `list` that carries a marker in one of
/// `fields`, else the newest one; `what` and the choice go to stderr.
fn newest(what: &str, list: Vec<Value>, fields: &[&str]) -> Value {
    let marked = list
        .iter()
        .filter(|v| carries_marker(v, fields))
        .max_by_key(|v| id(v));
    let (chosen, how) = match marked {
        Some(v) => (v, "newest with marker"),
        None => (
            list.iter().max_by_key(|v| id(v)).unwrap_or_else(|| {
                panic!("{what}: the test account holds none to take values from")
            }),
            "newest, no marker found",
        ),
    };
    eprintln!("PICK\t{what}\tid {}\t{how}", id(chosen));
    chosen.clone()
}

/// `SDRS-GATE-<digits>` as found in `v`'s text fields.
fn marker_of(v: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|f| {
        let s = v[*f].as_str()?;
        let at = s.find(MARKER_PREFIX)?;
        let digits: String = s[at + MARKER_PREFIX.len()..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        Some(format!("{MARKER_PREFIX}{digits}"))
    })
}

/// The facts of part 2 from existing data, and the marker of the chosen
/// invoice (what the `fulltext_search` queries look for).
pub(crate) async fn facts(ctx: &Ctx) -> (Facts, Option<String>) {
    let invoice = newest(
        "Invoice",
        get_all(ctx, "/Invoice?limit=1000").await,
        &["header", "customerInternalNote"],
    );
    let credit_note = newest(
        "CreditNote",
        get_all(ctx, "/CreditNote?limit=1000").await,
        &["header", "customerInternalNote"],
    );
    let voucher = newest(
        "Voucher",
        get_all(ctx, "/Voucher?limit=1000").await,
        &["description", "supplierName"],
    );
    let pos_path = format!(
        "/VoucherPos?voucher[id]={}&voucher[objectName]=Voucher",
        id(&voucher)
    );
    let voucher_pos = get_all(ctx, &pos_path)
        .await
        .into_iter()
        .next()
        .unwrap_or(Value::Null);
    let contact = newest(
        "Contact",
        get_all(ctx, "/Contact?depth=1&limit=1000").await,
        &["name", "familyname"],
    );
    let address_path = format!(
        "/ContactAddress?contact[id]={}&contact[objectName]=Contact",
        id(&contact)
    );
    let address = get_all(ctx, &address_path)
        .await
        .into_iter()
        .next()
        .unwrap_or(Value::Null);
    let transaction = newest(
        "Transaction",
        get_all(
            ctx,
            &format!(
                "/Transaction?checkAccount[id]={BANK}&checkAccount[objectName]=CheckAccount&limit=1000"
            ),
        )
        .await,
        &["payeePayerName", "paymtPurpose"],
    );
    let marker = marker_of(&invoice, &["header", "customerInternalNote"]);
    let invoice_number = invoice["invoiceNumber"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    (
        Facts {
            invoice,
            invoice_number,
            credit_note,
            voucher,
            voucher_pos,
            contact,
            address,
            transaction,
        },
        marker,
    )
}

/// `list_contacts_by_name` against a substring that matches more than the 100
/// contacts an unpaged answer holds, if the account has one: the walk must
/// equal a single `limit=1000` page (the count is below 1000 then).
pub(crate) async fn by_name(g: &mut Gate<'_>) {
    const N: &str = "ContactQuery";
    let ctx = g.ctx;
    let mut best: Option<(&str, usize)> = None;
    for needle in ["SDRS", "GmbH", "Test", "e", "a", " "] {
        match ctx.c.list_contacts_by_name(needle).await {
            Ok(list) => {
                eprintln!("LOOKUP\tlist_contacts_by_name\t{needle:?}\t{}", list.len());
                if best.is_none_or(|(_, n)| list.len() > n) {
                    best = Some((needle, list.len()));
                }
            }
            Err(e) => {
                g.rec.query(
                    N,
                    "list_contacts_by_name",
                    Expect::Wider,
                    100,
                    Err::<usize, _>(e),
                    "",
                );
                return;
            }
        }
    }
    let (needle, walked) = best.expect("at least one needle was tried");
    let single = ctx
        .c
        .list_contacts(&ContactQuery {
            name: Some(needle.to_owned()),
            page: Some(Page::new(1000, 0).unwrap()),
            ..Default::default()
        })
        .await
        .map(|l| l.len());
    let note = format!("needle {needle:?}: walked {walked}, single limit=1000 page {single:?}");
    g.rec.query(
        N,
        "list_contacts_by_name",
        Expect::Wider,
        100,
        Ok::<usize, String>(walked),
        &note,
    );
    if walked <= 100 {
        eprintln!(
            "NOTE\tno needle matches more than 100 organisations: the paging is unproven here ({note})"
        );
    } else if walked < 1000 {
        assert_eq!(
            single.ok(),
            Some(walked),
            "the walk lost or doubled contacts"
        );
    }
}
