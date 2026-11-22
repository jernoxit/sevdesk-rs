//! `filters_followup`: a targeted READ-ONLY follow-up to `filters_all`. The
//! automatic classes of that survey are too coarse for date bounds (inclusive
//! or exclusive, time zone, granularity), numbers, flags, orderings and
//! completeness (all returned objects match, but are all matching objects
//! returned?). Every answer is compared with the set a stated predicate would
//! select, see `followup_hyp.rs`. Output: stderr lines
//! `FOLLOWUP\t<section>\t...`; no value of the account is printed, only field
//! names, forms, counts and codes.
//!
//! `just test-live --test shape_survey filters_followup`

use super::filter_cases::{Aux, Expect, at, cases, cents, id_of, parse_dt, str_of, trailing_int};
use super::filter_table::{ENDPOINTS, Kind, SPECS};
use super::filters::fetch;
use super::followup_hyp::{
    Hyp, date_hyps, dist, number_hyps, pattern, pred, report, sorted_by, time_of_day,
};
use super::harness::{Ctx, SERIAL};
use super::verify_test_account;
use serde_json::Value;
use std::collections::BTreeMap;

type Pairs = Vec<(String, String)>;
type Lists = BTreeMap<&'static str, Vec<Value>>;

fn one(k: &str, v: impl Into<String>) -> Pairs {
    vec![(k.to_owned(), v.into())]
}

async fn get_list(ctx: &Ctx, endpoint: &str, pairs: &Pairs) -> Option<Vec<Value>> {
    match fetch(ctx, &format!("/{endpoint}"), pairs).await {
        Ok(l) => Some(l),
        Err(e) => {
            let e: String = e.chars().take(100).collect();
            eprintln!("FOLLOWUP\terror\t{endpoint}\t{e}");
            None
        }
    }
}

// ---------------------------------------------------------------- dates

#[derive(Clone, Copy, PartialEq)]
enum Forms {
    Unix,
    Strings,
    Day,
}

/// (endpoint, filter, field the bounds are taken from, forms to send)
const DATE_PROBES: &[(&str, &str, &str, Forms)] = &[
    ("Invoice", "updateAfter", "update", Forms::Unix),
    ("Invoice", "updateBefore", "update", Forms::Unix),
    ("Invoice", "createAfter", "create", Forms::Unix),
    ("Invoice", "createBefore", "create", Forms::Unix),
    ("Invoice", "startDate", "invoiceDate", Forms::Unix),
    ("Invoice", "endDate", "invoiceDate", Forms::Unix),
    ("Invoice", "day", "invoiceDate", Forms::Day),
    ("CreditNote", "startDate", "creditNoteDate", Forms::Unix),
    ("CreditNote", "endDate", "creditNoteDate", Forms::Unix),
    ("CreditNote", "day", "creditNoteDate", Forms::Day),
    ("Voucher", "startDate", "voucherDate", Forms::Unix),
    ("Voucher", "endDate", "voucherDate", Forms::Unix),
    ("Voucher", "createAfter", "create", Forms::Unix),
    ("Voucher", "createBefore", "create", Forms::Unix),
    ("Voucher", "updateAfter", "update", Forms::Unix),
    ("Voucher", "updateBefore", "update", Forms::Unix),
    ("Voucher", "startPayDate", "payDate", Forms::Unix),
    ("Voucher", "endPayDate", "payDate", Forms::Unix),
    ("Contact", "createAfter", "create", Forms::Unix),
    ("Contact", "createBefore", "create", Forms::Unix),
    ("Contact", "updateAfter", "update", Forms::Unix),
    ("Contact", "updateBefore", "update", Forms::Unix),
    (
        "CheckAccountTransaction",
        "startDate",
        "entryDate",
        Forms::Strings,
    ),
    (
        "CheckAccountTransaction",
        "endDate",
        "entryDate",
        Forms::Strings,
    ),
];

struct Form {
    label: &'static str,
    value: String,
    bound: i64,
}

fn forms_for(kind: Forms, text: &str, unix: i64) -> Vec<Form> {
    let midnight = unix - time_of_day(text);
    let offset = parse_dt(text).map_or(0, |t| i64::from(t.offset().whole_seconds()));
    let (y, m, d) = (&text[0..4], &text[5..7], &text[8..10]);
    let f = |label, value: String, bound| Form {
        label,
        value,
        bound,
    };
    match kind {
        Forms::Unix => vec![f("unix", unix.to_string(), unix)],
        Forms::Strings => vec![
            f("unix", unix.to_string(), unix),
            f("dd.mm.yyyy", format!("{d}.{m}.{y}"), midnight),
            f("yyyy-mm-dd", format!("{y}-{m}-{d}"), midnight),
            f("rfc3339", text.to_owned(), unix),
        ],
        Forms::Day => vec![
            f("unix local midnight", midnight.to_string(), midnight),
            f("unix local noon", (midnight + 43200).to_string(), midnight),
            f(
                "unix utc midnight",
                (midnight + offset).to_string(),
                midnight,
            ),
            f("unix end of day", (midnight + 86399).to_string(), midnight),
            f(
                "unix ms local midnight",
                (midnight * 1000).to_string(),
                midnight,
            ),
        ],
    }
}

async fn dates(ctx: &Ctx, lists: &Lists) {
    for (endpoint, filter, field, kind) in DATE_PROBES {
        let unf = &lists[endpoint];
        let mut inst: Vec<(i64, String)> = unf
            .iter()
            .filter_map(|o| {
                let s = str_of(at(o, field))?;
                Some((parse_dt(&s)?.unix_timestamp(), s))
            })
            .collect();
        inst.sort();
        inst.dedup_by_key(|(t, _)| *t);
        if inst.len() < 4 {
            eprintln!(
                "FOLLOWUP\tdate\t{endpoint}\t{filter}\tfewer than 4 distinct {field} instants"
            );
            continue;
        }
        for (k, idx) in [inst.len() / 4, inst.len() / 2, inst.len() * 3 / 4]
            .into_iter()
            .enumerate()
        {
            let (unix, text) = &inst[idx];
            let day = text[0..10].to_owned();
            for form in forms_for(*kind, text, *unix) {
                let Some(filt) = get_list(ctx, endpoint, &one(filter, form.value.clone())).await
                else {
                    continue;
                };
                let hyps = date_hyps(unf, form.bound, &day);
                eprintln!(
                    "FOLLOWUP\tdate\t{endpoint}\t{filter}\t{}\tbound{k}\t{}/{}\t{}",
                    form.label,
                    filt.len(),
                    unf.len(),
                    report(unf, &filt, &hyps)
                );
            }
        }
    }
}

// -------------------------------------------------------------- numbers

const NUMBER_PROBES: &[(&str, &str, &str)] = &[
    ("Invoice", "invoiceNumberGreater", "invoiceNumber"),
    ("Invoice", "invoiceNumberSmaller", "invoiceNumber"),
    ("CreditNote", "creditNoteNumberGreater", "creditNoteNumber"),
    ("CreditNote", "creditNoteNumberSmaller", "creditNoteNumber"),
    (
        "CreditNote",
        "creditNoteNumberNumberSmaller",
        "creditNoteNumber",
    ),
];

async fn numbers(ctx: &Ctx, lists: &Lists) {
    for endpoint in ["Invoice", "CreditNote"] {
        let field = if endpoint == "Invoice" {
            "invoiceNumber"
        } else {
            "creditNoteNumber"
        };
        let odd: Vec<String> = lists[endpoint]
            .iter()
            .filter_map(|o| match str_of(at(o, field)) {
                Some(s) if trailing_int(&s).is_some() => None,
                Some(s) => Some(pattern(&s)),
                None => Some("<null>".into()),
            })
            .collect();
        eprintln!(
            "FOLLOWUP\tnumber-odd\t{endpoint}\t{} of {} without a trailing integer: {odd:?}",
            odd.len(),
            lists[endpoint].len()
        );
    }
    for (endpoint, filter, field) in NUMBER_PROBES {
        let unf = &lists[endpoint];
        let mut nums: Vec<(i64, String)> = unf
            .iter()
            .filter_map(|o| {
                let s = str_of(at(o, field))?;
                Some((trailing_int(&s)?, s))
            })
            .collect();
        nums.sort();
        nums.dedup_by_key(|(n, _)| *n);
        for idx in [nums.len() / 4, nums.len() / 2, nums.len() * 3 / 4] {
            let (n, full) = nums[idx].clone();
            for (label, value) in [("full", full.clone()), ("digits", n.to_string())] {
                let Some(filt) = get_list(ctx, endpoint, &one(filter, value)).await else {
                    continue;
                };
                let hyps = number_hyps(unf, field, n, &full);
                eprintln!(
                    "FOLLOWUP\tnumber\t{endpoint}\t{filter}\t{label}\t{}/{}\t{}",
                    filt.len(),
                    unf.len(),
                    report(unf, &filt, &hyps)
                );
            }
        }
    }
}

// --------------------------------------------- completeness of text/ids

fn text_hyps(unf: &[Value], fields: &[&str], v: &str) -> Vec<Hyp> {
    let lv = v.to_lowercase();
    let mut out = Vec::new();
    let any = |o: &Value, f: &dyn Fn(&str) -> bool| {
        fields
            .iter()
            .any(|fl| str_of(at(o, fl)).is_some_and(|s| f(&s)))
    };
    out.push(pred(unf, "equals", |o| any(o, &|s| s == v)));
    out.push(pred(unf, "contains (case-sensitive)", |o| {
        any(o, &|s| s.contains(v))
    }));
    out.push(pred(unf, "contains (case-insensitive)", |o| {
        any(o, &|s| s.to_lowercase().contains(&lv))
    }));
    out.push(pred(unf, "starts with", |o| any(o, &|s| s.starts_with(v))));
    out.push(pred(unf, "starts with (case-insensitive)", |o| {
        any(o, &|s| s.to_lowercase().starts_with(&lv))
    }));
    out
}

async fn completeness(ctx: &Ctx, lists: &Lists) {
    let mut aux = Aux::default();
    for spec in SPECS {
        if !matches!(
            spec.kind,
            Kind::Exact | Kind::Text | Kind::Prefix | Kind::Ref(_)
        ) {
            continue;
        }
        let unf = &lists[spec.endpoint];
        let Ok(cs) = cases(ctx, &mut aux, spec, unf).await else {
            continue;
        };
        let fields: Vec<&str> = spec.field.split('|').collect();
        for case in cs {
            let hyps = match &case.expect {
                Expect::Match { value, .. } => text_hyps(unf, &fields, value),
                Expect::Ref { id } => vec![pred(unf, "reference id equals", |o| {
                    str_of(at(o, fields[0])).as_deref() == Some(id.as_str())
                })],
                _ => continue,
            };
            let Some(filt) = get_list(ctx, spec.endpoint, &case.pairs).await else {
                continue;
            };
            eprintln!(
                "FOLLOWUP\tcomplete\t{}\t{}\t{}\t{}/{}\t{}",
                spec.endpoint,
                spec.filter,
                case.form,
                filt.len(),
                unf.len(),
                report(unf, &filt, &hyps)
            );
            // negative control (value that matches nothing) and, for an exact
            // value, the value minus its last character
            if let Expect::Match { value, .. } = &case.expect {
                let key = &case.pairs[0].0;
                if let Some(neg) = get_list(ctx, spec.endpoint, &one(key, "zzqq9xv")).await {
                    eprintln!(
                        "FOLLOWUP\tneg\t{}\t{}\t{}/{}",
                        spec.endpoint,
                        spec.filter,
                        neg.len(),
                        unf.len()
                    );
                }
                let mut cut = value.clone();
                cut.pop();
                if matches!(spec.kind, Kind::Exact)
                    && cut.chars().count() >= 2
                    && let Some(t) = get_list(ctx, spec.endpoint, &one(key, cut.clone())).await
                {
                    eprintln!(
                        "FOLLOWUP\ttrunc\t{}\t{}\t{}/{}\t{}",
                        spec.endpoint,
                        spec.filter,
                        t.len(),
                        unf.len(),
                        report(unf, &t, &text_hyps(unf, &fields, &cut))
                    );
                }
            }
        }
    }
}

/// Negative control for text filters the table does not run through
/// `completeness` (narrowing filters and linked-list filters).
async fn negatives(ctx: &Ctx, lists: &Lists) {
    for (endpoint, filter) in [
        ("Invoice", "fulltextSearch"),
        ("Voucher", "fulltextSearch"),
        ("Voucher", "searchCommentOrDescription"),
        ("Voucher", "commentLike"),
        ("Invoice", "headerStartsWith"),
        ("Invoice", "headerOrNumber"),
        ("CreditNote", "headerStartsWith"),
        ("CreditNote", "headerOrNumber"),
        ("Contact", "city"),
        ("Contact", "zip"),
        ("CheckAccountTransaction", "payeePayerName"),
    ] {
        if let Some(neg) = get_list(ctx, endpoint, &one(filter, "zzqq9xv")).await {
            eprintln!(
                "FOLLOWUP\tneg\t{endpoint}\t{filter}\t{}/{}",
                neg.len(),
                lists[endpoint].len()
            );
        }
    }
}

// ---------------------------------------------------------------- flags

async fn flag(ctx: &Ctx, lists: &Lists, endpoint: &str, key: &str, value: &str, hyps: &[Hyp]) {
    let unf = &lists[endpoint];
    let Some(filt) = get_list(ctx, endpoint, &one(key, value)).await else {
        return;
    };
    let extra: Vec<&str> = ["status", "invoiceType", "voucherType"]
        .into_iter()
        .filter(|f| unf.iter().any(|o| !at(o, f).is_null()))
        .collect();
    let dists: Vec<String> = extra.iter().map(|f| dist(&filt, f)).collect();
    eprintln!(
        "FOLLOWUP\tflag\t{endpoint}\t{key}={value}\t{}/{}\t{}\treturned {}",
        filt.len(),
        unf.len(),
        report(unf, &filt, hyps),
        dists.join(" ")
    );
}

fn set(f: &'static str) -> impl Fn(&Value) -> bool {
    move |o| !at(o, f).is_null()
}

async fn flags(ctx: &Ctx, lists: &Lists) {
    for (endpoint, fields) in [
        (
            "Invoice",
            vec!["status", "invoiceType", "sendType", "dunningLevel"],
        ),
        ("CreditNote", vec!["status", "enshrined"]),
        ("Voucher", vec!["status", "voucherType", "creditDebit"]),
        ("Contact", vec!["category"]),
    ] {
        let unf = &lists[endpoint];
        let d: Vec<String> = fields.iter().map(|f| dist(unf, f)).collect();
        eprintln!("FOLLOWUP\tdist\t{endpoint}\tunfiltered\t{}", d.join(" "));
    }
    let status750 = |u: &[Value]| {
        pred(u, "status == 750", |o| {
            str_of(&o["status"]).as_deref() == Some("750")
        })
    };
    let inv = &lists["Invoice"];
    flag(
        ctx,
        lists,
        "Invoice",
        "partiallyPaid",
        "true",
        &[status750(inv)],
    )
    .await;
    flag(
        ctx,
        lists,
        "CreditNote",
        "partiallyPaid",
        "true",
        &[status750(&lists["CreditNote"])],
    )
    .await;
    flag(
        ctx,
        lists,
        "CreditNote",
        "onlyEnshrined",
        "true",
        &[pred(
            &lists["CreditNote"],
            "enshrined set",
            set("enshrined"),
        )],
    )
    .await;
    flag(
        ctx,
        lists,
        "Voucher",
        "hasDocument",
        "true",
        &[pred(&lists["Voucher"], "document set", set("document"))],
    )
    .await;
    flag(
        ctx,
        lists,
        "Invoice",
        "onlyDunned",
        "true",
        &[pred(inv, "dunningLevel set", set("dunningLevel"))],
    )
    .await;
    for (key, value) in [
        ("showWkr", "all"),
        ("showWkr", "true"),
        ("showWkr", "1"),
        ("showMa", "all"),
        ("showMa", "true"),
        ("showMa", "1"),
        ("showAll", "true"),
        ("notdelinquent", "true"),
    ] {
        flag(ctx, lists, "Invoice", key, value, &[]).await;
    }
    // delinquent: status and "invoiceDate + timeToPay days before now"
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    if let Some(filt) = get_list(ctx, "Invoice", &one("delinquent", "true")).await {
        let got: std::collections::HashSet<String> = filt.iter().map(id_of).collect();
        let mut groups: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for o in inv {
            let due = str_of(&o["invoiceDate"])
                .and_then(|s| parse_dt(&s))
                .map(|t| t.unix_timestamp())
                .zip(str_of(&o["timeToPay"]).and_then(|s| s.parse::<i64>().ok()))
                .map(|(t, d)| t + d * 86400);
            let key = format!(
                "status={} due_passed={}",
                str_of(&o["status"]).unwrap_or_default(),
                due.map_or("?".to_owned(), |d| (d < now).to_string())
            );
            let e = groups.entry(key).or_default();
            e.1 += 1;
            e.0 += usize::from(got.contains(&id_of(o)));
        }
        for (k, (r, t)) in groups {
            eprintln!("FOLLOWUP\tdelinquent\tInvoice\t{k}\treturned {r}/{t}");
        }
    }
    // Voucher: other values of the categorical filters
    for (key, value) in [
        ("creditDebit", "C"),
        ("creditDebit", "D"),
        ("voucherType", "VOU"),
        ("voucherType", "RV"),
        ("noRv", "true"),
        ("withoutCatering", "true"),
        ("delinquent", "true"),
    ] {
        flag(ctx, lists, "Voucher", key, value, &[]).await;
    }
    let years: Vec<i32> = lists["Voucher"]
        .iter()
        .filter_map(|o| parse_dt(&str_of(&o["voucherDate"])?).map(|t| t.year()))
        .collect();
    if let Some(y) = years.first() {
        for yy in [y - 1, *y, y + 1] {
            flag(ctx, lists, "Voucher", "year", &yy.to_string(), &[]).await;
        }
    }
    // Contact: depth
    let contacts = &lists["Contact"];
    for depth in ["0", "1"] {
        if let Some(filt) = get_list(ctx, "Contact", &one("depth", depth)).await {
            let known: std::collections::HashSet<String> = contacts.iter().map(id_of).collect();
            let extras: Vec<&Value> = filt.iter().filter(|o| !known.contains(&id_of(o))).collect();
            eprintln!(
                "FOLLOWUP\tdepth\tdepth={depth}\t{}/{}\textras {} of which with parent {}",
                filt.len(),
                contacts.len(),
                extras.len(),
                extras.iter().filter(|o| !o["parent"].is_null()).count()
            );
        }
    }
}

// --------------------------------------------------------- id loops

/// Tries every contact id as a reference filter: does any value narrow to a
/// non-empty set?
async fn contact_loops(ctx: &Ctx, lists: &Lists) {
    let ids: Vec<String> = lists["Contact"].iter().map(id_of).collect();
    for (endpoint, key) in [
        ("Voucher", "contact"),
        ("Voucher", "contactOrObject"),
        ("Contact", "parent"),
    ] {
        let (mut hits, mut total, mut max, mut tried) = (0usize, 0usize, 0usize, 0usize);
        for id in &ids {
            let pairs = vec![
                (format!("{key}[id]"), id.clone()),
                (format!("{key}[objectName]"), "Contact".to_owned()),
            ];
            let Some(filt) = get_list(ctx, endpoint, &pairs).await else {
                continue;
            };
            tried += 1;
            hits += usize::from(!filt.is_empty());
            total += filt.len();
            max = max.max(filt.len());
        }
        eprintln!(
            "FOLLOWUP\tloop\t{endpoint}\t{key}[id]\ttried {tried} contact ids: {hits} non-empty, {total} objects in total, max {max}"
        );
    }
}

// ---------------------------------------------------------------- orders

async fn orders(ctx: &Ctx, lists: &Lists) {
    for endpoint in ["Invoice", "CreditNote", "Voucher", "Contact"] {
        eprintln!(
            "FOLLOWUP\torder\t{endpoint}\tdefault list\t{}",
            sorted_by(&lists[endpoint])
        );
    }
    let probes: &[(&str, &str, &[&str])] = &[
        ("Invoice", "orderByDebit", &["true"]),
        ("Invoice", "orderByDueTime", &["true"]),
        ("Invoice", "orderByDueDate", &["true"]),
        ("Invoice", "orderByInvoiceNumber", &["ASC", "DESC"]),
        ("CreditNote", "orderByCreditNoteNumber", &["ASC", "DESC"]),
        ("CreditNote", "orderByDebit", &["true"]),
        ("Voucher", "orderByDebit", &["true"]),
        ("Voucher", "orderByVoucherNumber", &["ASC", "DESC"]),
        ("Contact", "orderByCustomerNumber", &["ASC", "DESC"]),
    ];
    for (endpoint, key, values) in probes {
        let unf: Vec<String> = lists[endpoint].iter().map(id_of).collect();
        let mut first: Option<Vec<String>> = None;
        for v in *values {
            let Some(filt) = get_list(ctx, endpoint, &one(key, *v)).await else {
                continue;
            };
            let ids: Vec<String> = filt.iter().map(id_of).collect();
            let rev = first.as_ref().map(|f| {
                let mut r = f.clone();
                r.reverse();
                r == ids
            });
            eprintln!(
                "FOLLOWUP\torder\t{endpoint}\t{key}={v}\t{}/{}\tsame order as default {}\treverse of the first answer {rev:?}\t{}",
                ids.len(),
                unf.len(),
                ids == unf,
                sorted_by(&filt)
            );
            first.get_or_insert(ids);
        }
    }
}

// ------------------------------------------------- transaction credit/debit

async fn credit_debit(ctx: &Ctx, lists: &Lists) {
    let unf = &lists["CheckAccountTransaction"];
    let mut got: BTreeMap<&str, std::collections::HashSet<String>> = BTreeMap::new();
    for key in ["onlyCredit", "onlyDebit"] {
        if let Some(f) = get_list(ctx, "CheckAccountTransaction", &one(key, "true")).await {
            got.insert(key, f.iter().map(id_of).collect());
        }
    }
    let mut tab: BTreeMap<String, usize> = BTreeMap::new();
    for o in unf {
        let id = id_of(o);
        let sign = match str_of(&o["amount"]).and_then(|s| cents(&s)) {
            Some(c) if c > 0 => "positive",
            Some(c) if c < 0 => "negative",
            Some(_) => "zero",
            None => "none",
        };
        let key = format!(
            "amount {sign}; status {}; credit {}; debit {}",
            str_of(&o["status"]).unwrap_or_default(),
            got.get("onlyCredit").is_some_and(|s| s.contains(&id)),
            got.get("onlyDebit").is_some_and(|s| s.contains(&id)),
        );
        *tab.entry(key).or_default() += 1;
    }
    for (k, n) in tab {
        eprintln!("FOLLOWUP\tcreditdebit\t{k}\t{n}");
    }
}

#[tokio::test]
#[ignore = "reads the whole sevDesk test account"]
async fn filters_followup() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping filters_followup");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ctx = Ctx::new(&token, "shape-survey".into());
    verify_test_account(&ctx).await;
    let mut lists = Lists::new();
    for endpoint in ENDPOINTS {
        let list = get_list(&ctx, endpoint, &Vec::new())
            .await
            .unwrap_or_default();
        eprintln!("FOLLOWUP\tlist\t{endpoint}\t{}", list.len());
        lists.insert(endpoint, list);
    }
    credit_debit(&ctx, &lists).await;
    flags(&ctx, &lists).await;
    numbers(&ctx, &lists).await;
    dates(&ctx, &lists).await;
    orders(&ctx, &lists).await;
    negatives(&ctx, &lists).await;
    completeness(&ctx, &lists).await;
    contact_loops(&ctx, &lists).await;
    eprintln!("FOLLOWUP\tdone");
}
