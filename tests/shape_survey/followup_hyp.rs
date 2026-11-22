//! `filters_followup`, part 1: hypothesis checking. A filter's answer is compared
//! with the set a stated predicate would select over the unfiltered list; a
//! hypothesis FITS only when the two sets are EQUAL (not merely "all returned
//! objects satisfy it"). Output carries hypothesis names (field, operator,
//! shift) and counts, never a value of the account.

use super::filter_cases::{at, day_of, id_of, parse_dt, str_of};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// A named predicate: one bool per object of the unfiltered list.
pub(crate) type Hyp = (String, Vec<bool>);

type NumOp = (&'static str, fn(i64, i64) -> bool);
type DayOp = (&'static str, fn(&str, &str) -> bool);

const NUM_OPS: [NumOp; 4] = [
    (">", |a, b| a > b),
    (">=", |a, b| a >= b),
    ("<", |a, b| a < b),
    ("<=", |a, b| a <= b),
];

const DAY_OPS: [DayOp; 5] = [
    (">", |a, b| a > b),
    (">=", |a, b| a >= b),
    ("<", |a, b| a < b),
    ("<=", |a, b| a <= b),
    ("==", |a, b| a == b),
];

/// Bound shifts in seconds tried against every date field.
const SHIFTS: [i64; 5] = [-7200, -3600, 0, 3600, 7200];

pub(crate) fn pred(unf: &[Value], name: &str, f: impl Fn(&Value) -> bool) -> Hyp {
    (name.to_owned(), unf.iter().map(f).collect())
}

/// Which hypotheses reproduce the answer exactly; else the closest one.
pub(crate) fn report(unf: &[Value], filt: &[Value], hyps: &[Hyp]) -> String {
    let ids: Vec<String> = unf.iter().map(id_of).collect();
    let known: HashSet<&String> = ids.iter().collect();
    let got: HashSet<String> = filt.iter().map(id_of).collect();
    let outside = got.iter().filter(|i| !known.contains(i)).count();
    let mut fits: Vec<&str> = Vec::new();
    let mut best: (usize, &str) = (usize::MAX, "");
    for (name, v) in hyps {
        let diff = outside
            + ids
                .iter()
                .zip(v)
                .filter(|(i, b)| got.contains(*i) != **b)
                .count();
        if diff == 0 {
            fits.push(name);
        } else if diff < best.0 {
            best = (diff, name);
        }
    }
    if fits.is_empty() {
        format!("no fit; closest diff {} ({})", best.0, best.1)
    } else {
        let more = fits.len().saturating_sub(8);
        let shown: Vec<&str> = fits.into_iter().take(8).collect();
        format!(
            "FITS {}{}",
            shown.join(" | "),
            if more > 0 {
                format!(" (+{more} more)")
            } else {
                String::new()
            }
        )
    }
}

/// Seconds since local midnight, read off the `THH:MM:SS` of an RFC 3339 text.
pub(crate) fn time_of_day(s: &str) -> i64 {
    let t: Vec<i64> = s
        .get(11..19)
        .unwrap_or("00:00:00")
        .split(':')
        .map(|x| x.parse().unwrap_or(0))
        .collect();
    t.first().copied().unwrap_or(0) * 3600
        + t.get(1).copied().unwrap_or(0) * 60
        + t.get(2).copied().unwrap_or(0)
}

/// Every date-like top-level field (some object carries an RFC 3339 text) with
/// the unix time and local day of each object (`None`: null or not a date).
fn date_fields(unf: &[Value]) -> BTreeMap<String, Vec<Option<(i64, String)>>> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for o in unf {
        if let Some(m) = o.as_object() {
            for (k, v) in m {
                if str_of(v).and_then(|s| parse_dt(&s)).is_some() {
                    names.insert(k.clone());
                }
            }
        }
    }
    names
        .into_iter()
        .map(|k| {
            let col = unf
                .iter()
                .map(|o| {
                    let s = str_of(&o[k.as_str()])?;
                    Some((parse_dt(&s)?.unix_timestamp(), day_of(&s)))
                })
                .collect();
            (k, col)
        })
        .collect()
}

/// Hypotheses "field op bound+shift" and "local day of field op bound day" for
/// every date field.
pub(crate) fn date_hyps(unf: &[Value], bound: i64, day: &str) -> Vec<Hyp> {
    let mut out = Vec::new();
    for (name, col) in date_fields(unf) {
        for shift in SHIFTS {
            for (op, f) in NUM_OPS {
                let v = col
                    .iter()
                    .map(|x| x.as_ref().is_some_and(|(t, _)| f(*t, bound + shift)))
                    .collect();
                out.push((format!("{name} {op} bound{shift:+}s"), v));
            }
        }
        for (op, f) in DAY_OPS {
            let v = col
                .iter()
                .map(|x| x.as_ref().is_some_and(|(_, d)| f(d, day)))
                .collect();
            out.push((format!("{name} day {op} D"), v));
        }
    }
    out
}

/// Hypotheses on the integer a number ends in and on the whole text.
pub(crate) fn number_hyps(unf: &[Value], field: &str, n: i64, full: &str) -> Vec<Hyp> {
    let mut out = Vec::new();
    for (op, f) in NUM_OPS {
        out.push(pred(unf, &format!("trailing int {op} n"), |o| {
            str_of(at(o, field))
                .and_then(|s| super::filter_cases::trailing_int(&s))
                .is_some_and(|x| f(x, n))
        }));
        out.push(pred(
            unf,
            &format!("trailing int {op} n (none as 0)"),
            |o| {
                let x = str_of(at(o, field))
                    .and_then(|s| super::filter_cases::trailing_int(&s))
                    .unwrap_or(0);
                f(x, n)
            },
        ));
        out.push(pred(unf, &format!("text {op} full (lexicographic)"), |o| {
            str_of(at(o, field)).is_some_and(|s| {
                let c = s.as_str().cmp(full);
                match op {
                    ">" => c.is_gt(),
                    ">=" => c.is_ge(),
                    "<" => c.is_lt(),
                    _ => c.is_le(),
                }
            })
        }));
    }
    out
}

/// Digits become `9`, letters `a`: the shape of a number, not its value.
pub(crate) fn pattern(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_digit() {
                '9'
            } else if c.is_alphabetic() {
                'a'
            } else {
                c
            }
        })
        .collect()
}

/// `{value: count}` of a categorical (code) field; never use on free text.
pub(crate) fn dist(objs: &[Value], field: &str) -> String {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for o in objs {
        let k = str_of(at(o, field)).unwrap_or_else(|| "null".into());
        *m.entry(k).or_insert(0) += 1;
    }
    let parts: Vec<String> = m.iter().map(|(k, c)| format!("{k}={c}")).collect();
    format!("{field}:{{{}}}", parts.join(","))
}

enum Key {
    Num(f64),
    Text(String),
}

fn key_column(unf: &[Value], ids: &[usize], field: &str, mode: u8) -> Option<Vec<Option<Key>>> {
    let col: Vec<Option<Key>> = ids
        .iter()
        .map(|i| {
            let s = str_of(at(&unf[*i], field))?;
            match mode {
                0 => Some(match parse_dt(&s) {
                    Some(t) => Key::Num(t.unix_timestamp() as f64),
                    None => s.parse::<f64>().map_or(Key::Text(s), Key::Num),
                }),
                _ => super::filter_cases::trailing_int(&s).map(|n| Key::Num(n as f64)),
            }
        })
        .collect();
    let nums = col
        .iter()
        .filter(|k| matches!(k, Some(Key::Num(_))))
        .count();
    let texts = col
        .iter()
        .filter(|k| matches!(k, Some(Key::Text(_))))
        .count();
    (nums == 0 || texts == 0).then_some(col)
}

fn monotone(col: &[Option<Key>]) -> (bool, bool, usize) {
    let seq: Vec<&Key> = col.iter().flatten().collect();
    let cmp = |a: &Key, b: &Key| match (a, b) {
        (Key::Num(x), Key::Num(y)) => x.partial_cmp(y),
        (Key::Text(x), Key::Text(y)) => Some(x.cmp(y)),
        _ => None,
    };
    let asc = seq
        .windows(2)
        .all(|w| cmp(w[0], w[1]).is_some_and(|c| c.is_le()));
    let desc = seq
        .windows(2)
        .all(|w| cmp(w[0], w[1]).is_some_and(|c| c.is_ge()));
    let distinct: BTreeSet<String> = seq
        .iter()
        .map(|k| match k {
            Key::Num(x) => format!("{x}"),
            Key::Text(t) => t.clone(),
        })
        .collect();
    (asc, desc, distinct.len())
}

/// Top-level fields (and the trailing integer of text fields, `#int`) by which
/// the list `objs` is sorted ascending / descending, ties allowed.
pub(crate) fn sorted_by(objs: &[Value]) -> String {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for o in objs {
        if let Some(m) = o.as_object() {
            for (k, v) in m {
                if matches!(v, Value::String(_) | Value::Number(_)) {
                    names.insert(k.clone());
                }
            }
        }
    }
    let ids: Vec<usize> = (0..objs.len()).collect();
    let (mut asc, mut desc) = (Vec::new(), Vec::new());
    for f in &names {
        for (mode, suffix) in [(0u8, ""), (1u8, "#int")] {
            let Some(col) = key_column(objs, &ids, f, mode) else {
                continue;
            };
            let (a, d, distinct) = monotone(&col);
            if distinct < 3 {
                continue;
            }
            if a {
                asc.push(format!("{f}{suffix}"));
            }
            if d {
                desc.push(format!("{f}{suffix}"));
            }
        }
    }
    format!("asc by [{}]; desc by [{}]", asc.join(","), desc.join(","))
}
