//! `filters_all`, part 2: compare the filtered answer with the unfiltered list and
//! classify the filter. Notes carry counts and categorical codes only, never a
//! value of the account.

use super::filter_cases::{Case, Expect, Mode, at, cents, id_of, parse_dt, str_of, trailing_int};
use super::filter_table::{DateOp, Spec};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use time::OffsetDateTime;

pub(crate) struct Verdict {
    pub(crate) class: &'static str,
    pub(crate) note: String,
}

fn v(class: &'static str, note: impl Into<String>) -> Verdict {
    Verdict {
        class,
        note: note.into(),
    }
}

/// Fields whose values are codes, not personal data: shown in distributions.
const CATEGORICAL: &[&str] = &[
    "status",
    "invoiceType",
    "sendType",
    "voucherType",
    "creditDebit",
    "dunningLevel",
];

fn fields(spec: &Spec) -> Vec<&str> {
    spec.field.split('|').filter(|f| !f.is_empty()).collect()
}

fn distribution(objs: &[Value], field: &str) -> String {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for o in objs {
        let x = at(o, field);
        let cat = match x {
            Value::Null => "null".to_owned(),
            Value::Object(_) => "obj".to_owned(),
            _ if CATEGORICAL.contains(&field) => str_of(x).unwrap_or_else(|| "empty".into()),
            _ => "set".to_owned(),
        };
        *m.entry(cat).or_insert(0) += 1;
    }
    let parts: Vec<String> = m.iter().map(|(k, c)| format!("{k}={c}")).collect();
    format!("{field}:{{{}}}", parts.join(","))
}

fn level(mode: Mode, s: &str, value: &str) -> u8 {
    let (sl, vl) = (s.to_lowercase(), value.to_lowercase());
    if s == value {
        3
    } else if mode == Mode::Prefix {
        if s.starts_with(value) {
            2
        } else {
            u8::from(sl.starts_with(&vl))
        }
    } else if s.contains(value) {
        2
    } else {
        u8::from(sl.contains(&vl))
    }
}

/// Compares an instant (or just its calendar day) with the bound.
fn date_ok(op: DateOp, t: OffsetDateTime, at: OffsetDateTime) -> bool {
    match op {
        DateOp::Ge => t >= at,
        DateOp::Gt => t > at,
        DateOp::Le => t <= at,
        DateOp::Lt => t < at,
        DateOp::Day => day_of_instant(t) == day_of_instant(at),
    }
}

fn day_of_instant(t: OffsetDateTime) -> (i32, u8, u8) {
    (t.year(), t.month() as u8, t.day())
}

fn date_ok_by_day(op: DateOp, t: OffsetDateTime, at: OffsetDateTime) -> bool {
    let (a, b) = (day_of_instant(t), day_of_instant(at));
    match op {
        DateOp::Ge => a >= b,
        DateOp::Gt => a > b,
        DateOp::Le => a <= b,
        DateOp::Lt => a < b,
        DateOp::Day => a == b,
    }
}

fn natural_key(s: &str) -> (i64, String) {
    (trailing_int(s).unwrap_or(i64::MIN), s.to_owned())
}

pub(crate) fn judge(spec: &Spec, case: &Case, unf: &[Value], filt: &[Value]) -> Verdict {
    let unf_ids: Vec<String> = unf.iter().map(id_of).collect();
    let filt_ids: Vec<String> = filt.iter().map(id_of).collect();
    let unf_set: HashSet<&String> = unf_ids.iter().collect();
    let filt_set: HashSet<&String> = filt_ids.iter().collect();
    let same_set = unf_set == filt_set && unf_ids.len() == filt_ids.len();
    let subset = filt_set.is_subset(&unf_set);
    let new_ids = filt_set.difference(&unf_set).count();
    let mut extra = String::new();
    if new_ids > 0 {
        extra = format!("; {new_ids} returned objects are not in the unfiltered list");
    }
    if let Expect::Match {
        all_match: true, ..
    } = case.expect
        && same_set
    {
        return v(
            "untestable",
            "every object carries the value: equal result cannot tell ignored from exact",
        );
    }
    if same_set && !matches!(case.expect, Expect::Order) {
        return v("ignored", "result equals the unfiltered list");
    }
    let fs = fields(spec);
    let n = filt.len();
    let mut verdict = match &case.expect {
        Expect::Match { value, mode, .. } => {
            if filt.is_empty() {
                return v(
                    "wrong",
                    "empty result although the source value occurs in the data",
                );
            }
            let mut best: Option<(u8, &str, usize)> = None;
            for f in &fs {
                let lv: Vec<u8> = filt
                    .iter()
                    .map(|o| str_of(at(o, f)).map_or(0, |s| level(*mode, &s, value)))
                    .collect();
                let min = lv.iter().copied().min().unwrap_or(0);
                let eq = lv.iter().filter(|l| **l == 3).count();
                if best.is_none_or(|(b, _, _)| min > b) {
                    best = Some((min, f, eq));
                }
            }
            let (min, f, eq) = best.unwrap_or((0, "", 0));
            let prefix = *mode == Mode::Prefix;
            match min {
                3 => v(
                    "exact",
                    format!("all {n} returned objects equal the value in {f}"),
                ),
                2 => v(
                    if prefix { "prefix" } else { "substring" },
                    format!("all {n} contain the value in {f}, {eq} equal it (case-sensitive)"),
                ),
                1 => v(
                    if prefix { "prefix" } else { "substring" },
                    format!("all {n} match in {f} only case-insensitively, {eq} equal it"),
                ),
                _ => v(
                    "wrong",
                    format!("some of {n} returned objects do not match the value in {f}"),
                ),
            }
        }
        Expect::Ref { id } => {
            let f = fs.first().copied().unwrap_or("");
            if filt.is_empty() {
                return v("wrong", "empty result although the id occurs in the data");
            }
            let bad = filt
                .iter()
                .filter(|o| str_of(at(o, f)).as_deref() != Some(id))
                .count();
            if bad == 0 {
                v(
                    "exact",
                    format!("all {n} returned objects reference the id in {f}"),
                )
            } else {
                v(
                    "wrong",
                    format!("{bad} of {n} returned objects do not reference the id in {f}"),
                )
            }
        }
        Expect::Date { op, at: bound } => {
            if filt.is_empty() {
                return v("wrong", "empty result for a bound inside the data range");
            }
            let mut fits = Vec::new();
            for f in &fs {
                let ts: Vec<OffsetDateTime> = filt
                    .iter()
                    .filter_map(|o| str_of(at(o, f)).and_then(|s| parse_dt(&s)))
                    .collect();
                if ts.len() != n {
                    continue;
                }
                let by_instant = ts.iter().all(|t| date_ok(*op, *t, *bound));
                let by_day = ts.iter().all(|t| date_ok_by_day(*op, *t, *bound));
                let boundary = ts.contains(bound);
                let boundary_day = ts
                    .iter()
                    .any(|t| day_of_instant(*t) == day_of_instant(*bound));
                if by_instant {
                    fits.push(format!(
                        "{f}: instant fit, boundary instant {}",
                        yn(boundary)
                    ));
                } else if by_day {
                    fits.push(format!(
                        "{f}: day-granular fit, boundary day {}",
                        yn(boundary_day)
                    ));
                }
            }
            if fits.is_empty() {
                v(
                    "wrong",
                    format!("no date field fits all {n} returned objects"),
                )
            } else {
                v("range", fits.join("; "))
            }
        }
        Expect::Year(y) => {
            let f = fs.first().copied().unwrap_or("");
            let bad = filt
                .iter()
                .filter(|o| {
                    str_of(at(o, f))
                        .and_then(|s| parse_dt(&s))
                        .map(|t| t.year())
                        != Some(*y)
                })
                .count();
            if n > 0 && bad == 0 {
                v(
                    "exact",
                    format!("all {n} returned objects have the year in {f}"),
                )
            } else {
                v(
                    "wrong",
                    format!("{bad} of {n} returned objects have another year in {f}"),
                )
            }
        }
        Expect::Month(m) => {
            let f = fs.first().copied().unwrap_or("");
            let bad = filt
                .iter()
                .filter(|o| {
                    str_of(at(o, f))
                        .and_then(|s| parse_dt(&s))
                        .map(|t| t.month() as u8)
                        != Some(*m)
                })
                .count();
            if n > 0 && bad == 0 {
                v(
                    "exact",
                    format!("all {n} returned objects have the month in {f}"),
                )
            } else {
                v(
                    "wrong",
                    format!("{bad} of {n} returned objects have another month in {f}"),
                )
            }
        }
        Expect::Flag => {
            let dist = fs
                .first()
                .map(|f| format!("; returned {}", distribution(filt, f)));
            let dist = dist.unwrap_or_default();
            if filt.is_empty() {
                v("narrows", format!("empty result{dist}"))
            } else if subset {
                v(
                    "narrows",
                    format!("subset of the unfiltered list, no per-object predicate{dist}"),
                )
            } else if unf_set.is_subset(&filt_set) {
                v("widens", format!("superset of the unfiltered list{dist}"))
            } else {
                v("changed", format!("neither subset nor superset{dist}"))
            }
        }
        Expect::Order => order_verdict(
            &fs,
            &case.pairs,
            unf_ids.as_slice(),
            &filt_ids,
            filt,
            same_set,
        ),
        Expect::Amount { ge, cents: bound } => {
            let mut fits = Vec::new();
            for f in &fs {
                let cs: Vec<i64> = filt
                    .iter()
                    .filter_map(|o| str_of(at(o, f)).and_then(|s| cents(&s)))
                    .collect();
                if cs.len() != n || n == 0 {
                    continue;
                }
                let ok = cs.iter().all(|c| if *ge { c >= bound } else { c <= bound });
                let boundary = cs.iter().any(|c| c == bound);
                if ok {
                    fits.push(format!("{f}: fit, boundary {}", yn(boundary)));
                }
            }
            if fits.is_empty() {
                v(
                    "wrong",
                    format!("no amount field fits all {n} returned objects"),
                )
            } else {
                v("range", fits.join("; "))
            }
        }
        Expect::NumCmp { greater, n: bound } => {
            let f = fs.first().copied().unwrap_or("");
            let ns: Vec<i64> = filt
                .iter()
                .filter_map(|o| str_of(at(o, f)).and_then(|s| trailing_int(&s)))
                .collect();
            let ok = !ns.is_empty()
                && ns.len() == n
                && ns
                    .iter()
                    .all(|x| if *greater { x >= bound } else { x <= bound });
            let boundary = ns.iter().any(|x| x == bound);
            if ok {
                v(
                    "range",
                    format!("{f}: trailing integers fit, boundary {}", yn(boundary)),
                )
            } else {
                v(
                    "wrong",
                    format!(
                        "trailing integers of {f} do not all fit ({} of {n} parse)",
                        ns.len()
                    ),
                )
            }
        }
        Expect::Sign(positive) => {
            let f = fs.first().copied().unwrap_or("");
            let cs: Vec<i64> = filt
                .iter()
                .filter_map(|o| str_of(at(o, f)).and_then(|s| cents(&s)))
                .collect();
            let ok = !cs.is_empty()
                && cs.len() == n
                && cs.iter().all(|c| if *positive { *c > 0 } else { *c < 0 });
            if ok {
                v(
                    "exact",
                    format!("all {n} returned {f} amounts have the sign"),
                )
            } else {
                v(
                    "wrong",
                    format!("not all {n} returned {f} amounts have the sign"),
                )
            }
        }
        Expect::Narrow { source } => {
            let inc = source
                .as_ref()
                .map(|s| {
                    format!(
                        "; source object {}",
                        if filt_ids.contains(s) {
                            "included"
                        } else {
                            "MISSING"
                        }
                    )
                })
                .unwrap_or_default();
            if filt.is_empty() {
                v("narrows", "empty result")
            } else if subset {
                v(
                    "narrows",
                    format!("subset of the unfiltered list, no per-object predicate{inc}"),
                )
            } else if unf_set.is_subset(&filt_set) {
                v("widens", format!("superset of the unfiltered list{inc}"))
            } else {
                v("changed", format!("neither subset nor superset{inc}"))
            }
        }
        Expect::Via { expected, text } => {
            let unf_exp: HashSet<&String> = unf_set
                .iter()
                .copied()
                .filter(|i| expected.contains(*i))
                .collect();
            let outside = filt_set.iter().filter(|i| !expected.contains(**i)).count();
            let class = if *text { "substring" } else { "exact" };
            if filt.is_empty() {
                v("wrong", "empty result although linked objects match")
            } else if outside > 0 {
                v(
                    "wrong",
                    format!(
                        "{outside} of {n} returned objects are not linked to a matching object"
                    ),
                )
            } else if filt_set == unf_exp {
                v(
                    class,
                    format!("equals the {} expected objects", expected.len()),
                )
            } else {
                v(
                    class,
                    format!("subset of the {} expected objects", unf_exp.len()),
                )
            }
        }
    };
    verdict.note.push_str(&extra);
    verdict
}

fn yn(b: bool) -> &'static str {
    if b { "included" } else { "excluded" }
}

fn order_verdict(
    fs: &[&str],
    pairs: &[(String, String)],
    unf_ids: &[String],
    filt_ids: &[String],
    filt: &[Value],
    same_set: bool,
) -> Verdict {
    if unf_ids == filt_ids {
        return v(
            "ignored",
            "same ids in the same order as the unfiltered list",
        );
    }
    if !same_set {
        return v(
            "changed",
            format!(
                "different object set: {} vs {}",
                filt_ids.len(),
                unf_ids.len()
            ),
        );
    }
    let sent = pairs.first().map(|(_, x)| x.as_str()).unwrap_or("");
    let Some(f) = fs.first() else {
        return v(
            "order",
            format!("same set, new order (sent {sent}); no field to check"),
        );
    };
    let keys: Vec<(i64, String)> = filt
        .iter()
        .map(|o| natural_key(&str_of(at(o, f)).unwrap_or_default()))
        .collect();
    let asc = keys.windows(2).all(|w| w[0] <= w[1]);
    let desc = keys.windows(2).all(|w| w[0] >= w[1]);
    let sorted = match (asc, desc) {
        (true, true) => "constant key",
        (true, false) => "ascending",
        (false, true) => "descending",
        (false, false) => "neither ascending nor descending",
    };
    v(
        "order",
        format!("same set, new order (sent {sent}); sorted by {f}: {sorted}"),
    )
}
