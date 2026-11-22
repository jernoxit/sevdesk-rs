//! `filters_followup_extra`: the questions the first follow-up left open. Which
//! field does `day` act on, are the text filters case-sensitive, is
//! `payeePayerName` a substring or a prefix, are `city` / `zip` exact or
//! partial? Read-only, counts and field names only.
//!
//! `just test-live --test shape_survey filters_followup_extra`

use super::filter_cases::{Aux, Expect, at, cases, cents, id_of, parse_dt, str_of};
use super::filter_table::{ENDPOINTS, Kind, SPECS};
use super::filters::fetch;
use super::followup_hyp::{pred, report, time_of_day};
use super::harness::{Ctx, SERIAL};
use super::verify_test_account;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

type Pairs = Vec<(String, String)>;
type Op = (&'static str, fn(i64, i64) -> bool);

fn one(k: &str, v: impl Into<String>) -> Pairs {
    vec![(k.to_owned(), v.into())]
}

async fn count(ctx: &Ctx, endpoint: &str, pairs: &Pairs) -> Option<usize> {
    match fetch(ctx, &format!("/{endpoint}"), pairs).await {
        Ok(l) => Some(l.len()),
        Err(e) => {
            let e: String = e.chars().take(100).collect();
            eprintln!("EXTRA\terror\t{endpoint}\t{e}");
            None
        }
    }
}

/// Median instant of a date field: (unix seconds, text).
fn median(unf: &[Value], field: &str) -> Option<(i64, String)> {
    let mut v: Vec<(i64, String)> = unf
        .iter()
        .filter_map(|o| {
            let s = str_of(at(o, field))?;
            Some((parse_dt(&s)?.unix_timestamp(), s))
        })
        .collect();
    v.sort();
    v.get(v.len() / 2).cloned()
}

async fn day_probe(ctx: &Ctx, endpoint: &str, unf: &[Value]) {
    let mut fields: Vec<&str> = vec![
        "invoiceDate",
        "creditNoteDate",
        "deliveryDate",
        "deliveryDateUntil",
        "payDate",
        "sendDate",
        "enshrined",
        "create",
        "update",
    ];
    fields.retain(|f| unf.iter().any(|o| !at(o, f).is_null()));
    for f in fields {
        let Some((unix, text)) = median(unf, f) else {
            continue;
        };
        let midnight = unix - time_of_day(&text);
        for (label, v) in [("local midnight", midnight), ("instant", unix)] {
            if let Some(n) = count(ctx, endpoint, &one("day", v.to_string())).await {
                eprintln!(
                    "EXTRA\tday\t{endpoint}\tbound from {f}, {label}\t{n}/{}",
                    unf.len()
                );
            }
        }
    }
    // the due day: invoiceDate + timeToPay days
    let due: Vec<i64> = unf
        .iter()
        .filter_map(|o| {
            let t = parse_dt(&str_of(&o["invoiceDate"])?)?.unix_timestamp();
            let d = str_of(&o["timeToPay"])?.parse::<i64>().ok()?;
            Some(t + d * 86400)
        })
        .collect();
    let mut sorted = due.clone();
    sorted.sort_unstable();
    if let Some(mid) = sorted.get(sorted.len() / 2) {
        for (label, v) in [("due day, unix", *mid), ("due day +12h", *mid + 43200)] {
            if let Some(n) = count(ctx, endpoint, &one("day", v.to_string())).await {
                eprintln!(
                    "EXTRA\tday\t{endpoint}\t{label}\t{n}/{} (objects due that day: {})",
                    unf.len(),
                    due.iter().filter(|d| (*d - mid).abs() < 43200).count()
                );
            }
        }
    }
}

async fn case_probe(ctx: &Ctx, unfs: &BTreeMap<&'static str, Vec<Value>>) {
    let mut aux = Aux::default();
    for spec in SPECS {
        if !matches!(spec.kind, Kind::Exact | Kind::Text) {
            continue;
        }
        let unf = &unfs[spec.endpoint];
        let Ok(cs) = cases(ctx, &mut aux, spec, unf).await else {
            continue;
        };
        for case in cs {
            let Expect::Match { value, .. } = &case.expect else {
                continue;
            };
            if value.to_uppercase() == value.to_lowercase() {
                continue;
            }
            let key = &case.pairs[0].0;
            let orig = count(ctx, spec.endpoint, &one(key, value.clone())).await;
            let up = count(ctx, spec.endpoint, &one(key, value.to_uppercase())).await;
            let low = count(ctx, spec.endpoint, &one(key, value.to_lowercase())).await;
            eprintln!(
                "EXTRA\tcase\t{}\t{}\toriginal {orig:?} upper {up:?} lower {low:?}",
                spec.endpoint, spec.filter
            );
            // a middle substring (neither prefix nor suffix)
            let chars: Vec<char> = value.chars().collect();
            if matches!(spec.kind, Kind::Exact) && chars.len() >= 5 {
                let mid: String = chars[1..chars.len() - 1].iter().collect();
                let Ok(filt) =
                    fetch(ctx, &format!("/{}", spec.endpoint), &one(key, mid.clone())).await
                else {
                    continue;
                };
                let hyps = [
                    pred(unf, "contains", |o| {
                        str_of(at(o, spec.field)).is_some_and(|s| s.contains(&mid))
                    }),
                    pred(unf, "starts with", |o| {
                        str_of(at(o, spec.field)).is_some_and(|s| s.starts_with(&mid))
                    }),
                    pred(unf, "equals", |o| {
                        str_of(at(o, spec.field)).is_some_and(|s| s == mid)
                    }),
                ];
                eprintln!(
                    "EXTRA\tmiddle\t{}\t{}\t{}/{}\t{}",
                    spec.endpoint,
                    spec.filter,
                    filt.len(),
                    unf.len(),
                    report(unf, &filt, &hyps)
                );
            }
        }
    }
}

/// `city` / `zip`: exact, prefix or substring of the address field?
async fn address_probe(ctx: &Ctx, contacts: &[Value]) {
    let Ok(addresses) = fetch(ctx, "/ContactAddress", &Vec::new()).await else {
        return;
    };
    for field in ["city", "zip"] {
        let mut freq: BTreeMap<String, usize> = BTreeMap::new();
        for a in &addresses {
            if let Some(s) = str_of(&a[field]).filter(|s| s.chars().count() >= 4) {
                *freq.entry(s).or_default() += 1;
            }
        }
        let Some((value, _)) = freq.iter().max_by_key(|(_, c)| **c) else {
            continue;
        };
        let chars: Vec<char> = value.chars().collect();
        let variants = [
            (
                "minus last char",
                chars[..chars.len() - 1].iter().collect::<String>(),
            ),
            (
                "middle",
                chars[1..chars.len() - 1].iter().collect::<String>(),
            ),
            ("upper", value.to_uppercase()),
            ("lower", value.to_lowercase()),
        ];
        for (label, v) in variants {
            let linked = |f: &dyn Fn(&str) -> bool| -> HashSet<String> {
                addresses
                    .iter()
                    .filter(|a| str_of(&a[field]).is_some_and(|s| f(&s)))
                    .filter_map(|a| str_of(&a["contact"]))
                    .collect()
            };
            let (eq, pre, sub, eq_ci) = (
                linked(&|s| s == v),
                linked(&|s| s.starts_with(&v)),
                linked(&|s| s.contains(&v)),
                linked(&|s| s.to_lowercase() == v.to_lowercase()),
            );
            let Ok(filt) = fetch(ctx, "/Contact", &one(field, v.clone())).await else {
                continue;
            };
            let hyps = [
                pred(contacts, "equals", |o| eq.contains(&id_of(o))),
                pred(contacts, "equals (case-insensitive)", |o| {
                    eq_ci.contains(&id_of(o))
                }),
                pred(contacts, "starts with", |o| pre.contains(&id_of(o))),
                pred(contacts, "contains", |o| sub.contains(&id_of(o))),
            ];
            eprintln!(
                "EXTRA\taddress\t{field}\t{label}\t{}/{}\t{}",
                filt.len(),
                contacts.len(),
                report(contacts, &filt, &hyps)
            );
        }
    }
}

/// `startAmount` / `endAmount`: which amount field, which operator?
async fn amount_probe(ctx: &Ctx, endpoint: &str, unf: &[Value], bound_field: &str) {
    let mut names: Vec<String> = unf
        .first()
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    names.retain(|f| {
        let cs: Vec<Option<i64>> = unf
            .iter()
            .map(|o| str_of(&o[f.as_str()]).and_then(|s| cents(&s)))
            .collect();
        cs.iter()
            .flatten()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            >= 3
            && f.starts_with("sum")
    });
    let mut gross: Vec<i64> = unf
        .iter()
        .filter_map(|o| str_of(&o[bound_field]).and_then(|s| cents(&s)))
        .collect();
    gross.sort_unstable();
    gross.dedup();
    for filter in ["startAmount", "endAmount"] {
        for idx in [gross.len() / 4, gross.len() / 2, gross.len() * 3 / 4] {
            let c = gross[idx];
            let value = format!(
                "{}{}.{:02}",
                if c < 0 { "-" } else { "" },
                c.abs() / 100,
                c.abs() % 100
            );
            let Ok(filt) = fetch(ctx, &format!("/{endpoint}"), &one(filter, value)).await else {
                continue;
            };
            let mut hyps = Vec::new();
            for f in &names {
                let ops: [Op; 4] = [
                    (">", |a, b| a > b),
                    (">=", |a, b| a >= b),
                    ("<", |a, b| a < b),
                    ("<=", |a, b| a <= b),
                ];
                for (op, cmp) in ops {
                    hyps.push(pred(unf, &format!("{f} {op} bound"), |o| {
                        str_of(&o[f.as_str()])
                            .and_then(|s| cents(&s))
                            .is_some_and(|x| cmp(x, c))
                    }));
                }
            }
            eprintln!(
                "EXTRA\tamount\t{endpoint}\t{filter}\tbound from {bound_field}\t{}/{}\t{}",
                filt.len(),
                unf.len(),
                report(unf, &filt, &hyps)
            );
        }
    }
}

/// Date-only bounds on transactions: is the named day itself part of the
/// answer? Counts per position relative to the day, by `entryDate`.
async fn transaction_day_probe(ctx: &Ctx, unf: &[Value]) {
    let mut per_day: BTreeMap<String, usize> = BTreeMap::new();
    for o in unf {
        if let Some(s) = str_of(&o["entryDate"]) {
            *per_day.entry(s[0..10].to_owned()).or_default() += 1;
        }
    }
    let days: Vec<&String> = per_day
        .iter()
        .filter(|(_, n)| **n >= 3)
        .map(|(d, _)| d)
        .collect();
    let Some(day) = days.get(days.len() / 2) else {
        return;
    };
    for filter in ["startDate", "endDate"] {
        let Ok(filt) = fetch(
            ctx,
            "/CheckAccountTransaction",
            &one(filter, (*day).clone()),
        )
        .await
        else {
            continue;
        };
        let pos = |o: &Value| str_of(&o["entryDate"]).map(|s| s[0..10].cmp(day.as_str()));
        let count = |l: &[Value], want: std::cmp::Ordering| {
            l.iter().filter(|o| pos(o) == Some(want)).count()
        };
        eprintln!(
            "EXTRA\ttxday\t{filter}\tentries on the day: {} of {} unfiltered; returned before/on/after the day: {}/{}/{}; unfiltered before/on/after: {}/{}/{}",
            per_day[*day],
            unf.len(),
            count(&filt, std::cmp::Ordering::Less),
            count(&filt, std::cmp::Ordering::Equal),
            count(&filt, std::cmp::Ordering::Greater),
            count(unf, std::cmp::Ordering::Less),
            count(unf, std::cmp::Ordering::Equal),
            count(unf, std::cmp::Ordering::Greater),
        );
    }
}

#[tokio::test]
#[ignore = "reads the whole sevDesk test account"]
async fn filters_followup_extra() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping filters_followup_extra");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ctx = Ctx::new(&token, "shape-survey".into());
    verify_test_account(&ctx).await;
    let mut unfs: BTreeMap<&'static str, Vec<Value>> = BTreeMap::new();
    for endpoint in ENDPOINTS {
        let list = fetch(&ctx, &format!("/{endpoint}"), &Vec::new())
            .await
            .unwrap_or_default();
        unfs.insert(endpoint, list);
    }
    {
        for endpoint in ["Invoice", "CreditNote"] {
            day_probe(&ctx, endpoint, &unfs[endpoint]).await;
        }
        address_probe(&ctx, &unfs["Contact"]).await;
        case_probe(&ctx, &unfs).await;
    }
    for endpoint in ["CreditNote", "Voucher"] {
        for field in ["sumGross", "sumNet"] {
            amount_probe(&ctx, endpoint, &unfs[endpoint], field).await;
        }
    }
    transaction_day_probe(&ctx, &unfs["CheckAccountTransaction"]).await;
    eprintln!("EXTRA\tdone");
}
