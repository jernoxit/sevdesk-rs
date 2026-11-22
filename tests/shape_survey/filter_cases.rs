//! `filters_all`, part 1: from a filter row and the unfiltered list to the
//! requests to send (`Case`) and what each is expected to return (`Expect`).
//! Values come from the live data and are never printed: a case carries only a
//! value FORM (a constant description), not the value.

use super::filter_table::{DateOp, Kind, Spec, ViaMode};
use super::harness::Ctx;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Exact,
    Substring,
    Prefix,
}

pub(crate) enum Expect {
    Match {
        value: String,
        mode: Mode,
        /// Every object of the unfiltered list carries the value.
        all_match: bool,
    },
    Ref {
        id: String,
    },
    Date {
        op: DateOp,
        at: OffsetDateTime,
    },
    Year(i32),
    Month(u8),
    Flag,
    Order,
    Amount {
        ge: bool,
        cents: i64,
    },
    NumCmp {
        greater: bool,
        n: i64,
    },
    Sign(bool),
    Narrow {
        source: Option<String>,
    },
    Via {
        expected: HashSet<String>,
        text: bool,
    },
}

pub(crate) struct Case {
    pub(crate) form: String,
    pub(crate) pairs: Vec<(String, String)>,
    pub(crate) expect: Expect,
}

/// Lists fetched once per path (tags, contacts, positions, addresses).
#[derive(Default)]
pub(crate) struct Aux {
    lists: HashMap<String, Result<Vec<Value>, String>>,
}

impl Aux {
    async fn list(&mut self, ctx: &Ctx, path: &str) -> Result<&[Value], String> {
        if !self.lists.contains_key(path) {
            let got = super::filters::fetch(ctx, path, &[]).await;
            self.lists.insert(path.to_owned(), got);
        }
        match self.lists.get(path) {
            Some(Ok(v)) => Ok(v),
            Some(Err(e)) => Err(format!("list {path} failed: {e}")),
            None => Err(format!("list {path} missing")),
        }
    }
}

/// The scalar a JSON value stands for; objects stand for their `id`.
pub(crate) fn str_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Object(m) => m.get("id").and_then(str_of),
        _ => None,
    }
}

pub(crate) fn at<'a>(o: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(o, |v, k| &v[k])
}

pub(crate) fn id_of(o: &Value) -> String {
    str_of(&o["id"]).unwrap_or_default()
}

fn first_field(spec: &Spec) -> &str {
    spec.field.split('|').next().unwrap_or_default()
}

/// `contact[id]` -> `contact`; `tags[x][id]` -> `tags[0]`.
fn ref_prefix(filter: &str) -> String {
    filter
        .strip_suffix("[id]")
        .unwrap_or(filter)
        .replace("[x]", "[0]")
}

fn ref_pairs(filter: &str, id: &str, object_name: &str) -> Vec<(String, String)> {
    let p = ref_prefix(filter);
    vec![
        (format!("{p}[id]"), id.to_owned()),
        (format!("{p}[objectName]"), object_name.to_owned()),
    ]
}

fn one(filter: &str, value: String) -> Vec<(String, String)> {
    vec![(filter.to_owned(), value)]
}

/// Most frequent value that is NOT carried by every object (so it can narrow);
/// else the value all objects carry (`true`).
fn pick(counts: &BTreeMap<String, usize>, n: usize) -> Option<(String, bool)> {
    let mut best: Option<(&String, usize)> = None;
    for (v, &c) in counts {
        if c < n && best.is_none_or(|(_, b)| c > b) {
            best = Some((v, c));
        }
    }
    if let Some((v, _)) = best {
        return Some((v.clone(), false));
    }
    counts.keys().next().map(|v| (v.clone(), true))
}

fn count_values(objs: &[Value], field: &str) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for o in objs {
        if let Some(s) = str_of(at(o, field)) {
            *m.entry(s).or_insert(0) += 1;
        }
    }
    m
}

/// A strict substring (or prefix) of a text of some object, preferably one that
/// does not occur in every object. Returns (value, source object id).
fn pick_text(objs: &[Value], field: &str, prefix: bool) -> Option<(String, String)> {
    let min_len = if prefix { 3 } else { 4 };
    let mut fallback = None;
    for o in objs {
        let Some(s) = str_of(at(o, field)) else {
            continue;
        };
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < min_len {
            continue;
        }
        let sub: String = if prefix {
            chars.iter().take(6.min(chars.len() - 1)).collect()
        } else {
            chars.iter().skip(1).take(6.min(chars.len() - 2)).collect()
        };
        let count = objs
            .iter()
            .filter(|x| {
                str_of(at(x, field)).is_some_and(|t| {
                    if prefix {
                        t.starts_with(&sub)
                    } else {
                        t.contains(&sub)
                    }
                })
            })
            .count();
        let cand = (sub, id_of(o));
        if count < objs.len() {
            return Some(cand);
        }
        fallback.get_or_insert(cand);
    }
    fallback
}

pub(crate) fn parse_dt(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

pub(crate) fn day_of(s: &str) -> String {
    s.chars().take(10).collect()
}

fn instants(objs: &[Value], field: &str) -> Vec<(OffsetDateTime, String)> {
    objs.iter()
        .filter_map(|o| {
            let s = str_of(at(o, field))?;
            Some((parse_dt(&s)?, s))
        })
        .collect()
}

/// Cents of a decimal string such as `-100.32`, `238` or `43.04`.
pub(crate) fn cents(s: &str) -> Option<i64> {
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (int, frac) = rest.split_once('.').unwrap_or((rest, ""));
    if int.is_empty() || !int.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac.chars().all(|c| c.is_ascii_digit()) || frac.len() > 8 {
        return None;
    }
    let frac2: String = format!("{frac:0<2}").chars().take(2).collect();
    let v = int.parse::<i64>().ok()? * 100 + frac2.parse::<i64>().ok()?;
    Some(if neg { -v } else { v })
}

fn format_cents(c: i64) -> String {
    let sign = if c < 0 { "-" } else { "" };
    let a = c.abs();
    format!("{sign}{}.{:02}", a / 100, a % 100)
}

/// The integer a number ends in (`RE-1005` -> 1005).
pub(crate) fn trailing_int(s: &str) -> Option<i64> {
    let digits: String = s
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().ok()
}

/// Index into `sorted` for a lower bound (`ge`: excludes the smallest) or an
/// upper bound (excludes the largest); needs two distinct values.
fn bound_index(len: usize, lower: bool) -> usize {
    if lower { len / 2 } else { (len - 1) / 2 }
}

fn date_forms(at: OffsetDateTime) -> Vec<(String, String)> {
    let (y, m, d) = (at.year(), at.month() as u8, at.day());
    vec![
        ("unix seconds".into(), at.unix_timestamp().to_string()),
        ("dd.mm.yyyy".into(), format!("{d:02}.{m:02}.{y:04}")),
        ("yyyy-mm-dd".into(), format!("{y:04}-{m:02}-{d:02}")),
        ("rfc3339".into(), at.format(&Rfc3339).unwrap_or_default()),
    ]
}

/// The requests for one filter row, or the reason it cannot be tested.
pub(crate) async fn cases(
    ctx: &Ctx,
    aux: &mut Aux,
    spec: &Spec,
    unf: &[Value],
) -> Result<Vec<Case>, String> {
    let n = unf.len();
    let f = first_field(spec);
    let filter = spec.filter;
    match spec.kind {
        Kind::Absent(reason) => Err(reason.to_owned()),
        Kind::Exact => {
            let (v, all) = pick(&count_values(unf, f), n)
                .ok_or_else(|| format!("no object carries a non-empty {f}"))?;
            Ok(vec![Case {
                form: format!("exact copy of {f} (len {})", v.chars().count()),
                pairs: one(filter, v.clone()),
                expect: Expect::Match {
                    value: v,
                    mode: Mode::Exact,
                    all_match: all,
                },
            }])
        }
        Kind::Text | Kind::Prefix => {
            let prefix = matches!(spec.kind, Kind::Prefix);
            let (v, _) = pick_text(unf, f, prefix).ok_or_else(|| {
                format!(
                    "no {f} text of at least {} chars",
                    if prefix { 3 } else { 4 }
                )
            })?;
            let all = unf.iter().all(|o| {
                str_of(at(o, f)).is_some_and(|t| {
                    if prefix {
                        t.starts_with(&v)
                    } else {
                        t.contains(&v)
                    }
                })
            });
            Ok(vec![Case {
                form: format!(
                    "strict {} of {f} (len {})",
                    if prefix { "prefix" } else { "substring" },
                    v.chars().count()
                ),
                pairs: one(filter, v.clone()),
                expect: Expect::Match {
                    value: v,
                    mode: if prefix {
                        Mode::Prefix
                    } else {
                        Mode::Substring
                    },
                    all_match: all,
                },
            }])
        }
        Kind::Ref(default_name) => {
            let (id, _) = pick(&count_values(unf, f), n)
                .ok_or_else(|| format!("no object carries a {f} reference"))?;
            let name = unf
                .iter()
                .find(|o| str_of(at(o, f)).as_deref() == Some(&id))
                .and_then(|o| o[f]["objectName"].as_str().map(str::to_owned))
                .unwrap_or_else(|| default_name.to_owned());
            if name.is_empty() {
                return Err(format!("{f} carries no objectName and none is known"));
            }
            Ok(vec![Case {
                form: format!("id of a {f} reference + objectName {name}"),
                pairs: ref_pairs(filter, &id, &name),
                expect: Expect::Ref { id },
            }])
        }
        Kind::Date(op) => {
            let mut inst = instants(unf, f);
            if inst.len() < 2 {
                return Err(format!("fewer than two objects carry a {f} date"));
            }
            inst.sort_by_key(|(t, _)| *t);
            let chosen = if matches!(op, DateOp::Day) {
                let days: BTreeSet<String> = inst.iter().map(|(_, s)| day_of(s)).collect();
                if days.len() < 2 {
                    return Err(format!("all {f} dates fall on one day"));
                }
                let day = days.iter().nth(days.len() / 2).cloned().unwrap_or_default();
                inst.iter().find(|(_, s)| day_of(s) == day).map(|(t, _)| *t)
            } else {
                let mut d: Vec<OffsetDateTime> = inst.iter().map(|(t, _)| *t).collect();
                d.dedup();
                if d.len() < 2 {
                    return Err(format!("all {f} dates are the same instant"));
                }
                let lower = matches!(op, DateOp::Ge | DateOp::Gt);
                Some(d[bound_index(d.len(), lower)])
            };
            let at = chosen.ok_or("no date chosen")?;
            Ok(date_forms(at)
                .into_iter()
                .map(|(form, v)| Case {
                    form: format!("{form} of a {f} date"),
                    pairs: one(filter, v),
                    expect: Expect::Date { op, at },
                })
                .collect())
        }
        Kind::Year | Kind::Month => {
            let year = matches!(spec.kind, Kind::Year);
            let inst = instants(unf, f);
            let part =
                |t: &OffsetDateTime| -> i32 { if year { t.year() } else { t.month() as i32 } };
            let distinct: BTreeSet<i32> = inst.iter().map(|(t, _)| part(t)).collect();
            if distinct.len() < 2 {
                return Err(format!(
                    "all {f} dates share one {}",
                    if year { "year" } else { "month" }
                ));
            }
            let v = distinct
                .iter()
                .nth(distinct.len() / 2)
                .copied()
                .unwrap_or_default();
            Ok(vec![Case {
                form: format!(
                    "{} number of a {f} date",
                    if year { "year" } else { "month" }
                ),
                pairs: one(filter, v.to_string()),
                expect: if year {
                    Expect::Year(v)
                } else {
                    Expect::Month(u8::try_from(v).unwrap_or(0))
                },
            }])
        }
        Kind::Flag(value) => Ok(vec![Case {
            form: format!("constant {value}"),
            pairs: one(filter, value.to_owned()),
            expect: Expect::Flag,
        }]),
        Kind::Order(values) => Ok(values
            .iter()
            .map(|v| Case {
                form: format!("constant {v}"),
                pairs: one(filter, (*v).to_owned()),
                expect: Expect::Order,
            })
            .collect()),
        Kind::Amount(ge) => {
            let mut c: Vec<i64> = unf
                .iter()
                .filter_map(|o| str_of(at(o, f)).and_then(|s| cents(&s)))
                .collect();
            c.sort_unstable();
            c.dedup();
            if c.len() < 2 {
                return Err(format!("fewer than two distinct {f} amounts"));
            }
            let v = c[bound_index(c.len(), ge)];
            Ok(vec![Case {
                form: format!("decimal copy of a {f} amount"),
                pairs: one(filter, format_cents(v)),
                expect: Expect::Amount { ge, cents: v },
            }])
        }
        Kind::NumCmp(greater) => {
            let mut nums: Vec<(i64, String)> = unf
                .iter()
                .filter_map(|o| {
                    let s = str_of(at(o, f))?;
                    Some((trailing_int(&s)?, s))
                })
                .collect();
            nums.sort();
            nums.dedup_by_key(|(n, _)| *n);
            if nums.len() < 2 {
                return Err(format!("fewer than two distinct {f} numbers"));
            }
            let (num, full) = nums[bound_index(nums.len(), greater)].clone();
            Ok(vec![
                Case {
                    form: format!("full copy of a {f} (len {})", full.chars().count()),
                    pairs: one(filter, full),
                    expect: Expect::NumCmp { greater, n: num },
                },
                Case {
                    form: "trailing digits of a number".into(),
                    pairs: one(filter, num.to_string()),
                    expect: Expect::NumCmp { greater, n: num },
                },
            ])
        }
        Kind::Sign(positive) => Ok(vec![Case {
            form: "constant true".into(),
            pairs: one(filter, "true".into()),
            expect: Expect::Sign(positive),
        }]),
        Kind::Narrow => {
            let (v, source) = pick_text(unf, f, false)
                .ok_or_else(|| format!("no {f} text of at least 4 chars"))?;
            Ok(vec![Case {
                form: format!("strict substring of {f} (len {})", v.chars().count()),
                pairs: one(filter, v),
                expect: Expect::Narrow {
                    source: Some(source),
                },
            }])
        }
        Kind::NarrowRef(list, object_name) => {
            let ids: Vec<String> = aux
                .list(ctx, list)
                .await?
                .iter()
                .map(id_of)
                .filter(|i| !i.is_empty())
                .take(3)
                .collect();
            if ids.is_empty() {
                return Err(format!("GET {list} returned no object to take an id from"));
            }
            Ok(ids
                .iter()
                .enumerate()
                .map(|(k, id)| Case {
                    form: format!("id #{} of GET {list} + objectName {object_name}", k + 1),
                    pairs: ref_pairs(filter, id, object_name),
                    expect: Expect::Narrow { source: None },
                })
                .collect())
        }
        Kind::Via(list, link, mode) => {
            let linked: Vec<(String, String)> = aux
                .list(ctx, list)
                .await?
                .iter()
                .filter_map(|o| Some((str_of(at(o, spec.field))?, str_of(&o[link])?)))
                .collect();
            let unf_ids: HashSet<String> = unf.iter().map(id_of).collect();
            let text = matches!(mode, ViaMode::Text);
            let expected_for = |v: &str| -> HashSet<String> {
                linked
                    .iter()
                    .filter(|(x, _)| if text { x.contains(v) } else { x == v })
                    .map(|(_, parent)| parent.clone())
                    .filter(|p| unf_ids.contains(p))
                    .collect()
            };
            let candidates: Vec<String> = if text {
                linked
                    .iter()
                    .filter(|(x, _)| x.chars().count() >= 4)
                    .map(|(x, _)| {
                        let c: Vec<char> = x.chars().collect();
                        c.iter().skip(1).take(6.min(c.len() - 2)).collect()
                    })
                    .collect()
            } else {
                linked.iter().map(|(x, _)| x.clone()).collect()
            };
            let mut best: Option<(String, HashSet<String>)> = None;
            for v in candidates {
                let exp = expected_for(&v);
                let partial = !exp.is_empty() && exp.len() < n;
                let better = best.as_ref().is_none_or(|(_, b)| {
                    let b_partial = !b.is_empty() && b.len() < n;
                    (partial && !b_partial) || (partial == b_partial && exp.len() > b.len())
                });
                if better {
                    best = Some((v, exp));
                }
            }
            let (v, expected) = best.filter(|(_, e)| !e.is_empty()).ok_or_else(|| {
                format!(
                    "no {} value in GET {list} reaches an object of this list",
                    spec.field
                )
            })?;
            let (pairs, form) = match mode {
                ViaMode::Ref(name) => (
                    ref_pairs(filter, &v, name),
                    format!("id of a {} in GET {list} + objectName {name}", spec.field),
                ),
                ViaMode::Exact => (
                    one(filter, v.clone()),
                    format!(
                        "exact copy of {} in GET {list} (len {})",
                        spec.field,
                        v.chars().count()
                    ),
                ),
                ViaMode::Text => (
                    one(filter, v.clone()),
                    format!(
                        "strict substring of {} in GET {list} (len {})",
                        spec.field,
                        v.chars().count()
                    ),
                ),
            };
            Ok(vec![Case {
                form,
                pairs,
                expect: Expect::Via { expected, text },
            }])
        }
    }
}
