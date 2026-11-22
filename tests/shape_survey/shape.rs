//! Classifies JSON values by FORM and aggregates them per key.
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The TSV columns of the form counts, in output order.
const FORMS: [&str; 12] = [
    "null",
    "bool",
    "int",
    "float",
    "str_empty",
    "str_int",
    "str_decimal",
    "str_timestamp",
    "str_bool_word",
    "str_other",
    "object",
    "array",
];

/// Key-name fragments (lower case) whose values are never recorded, only forms.
const SENSITIVE: [&str; 13] = [
    "iban",
    "bic",
    "pin",
    "password",
    "token",
    "apikey",
    "email",
    "phone",
    "mobile",
    "fax",
    "taxnumber",
    "vatnumber",
    "smtp",
];

const MAX_EXAMPLES: usize = 3;
const EXAMPLE_LEN: usize = 40;

#[derive(Default)]
struct FormStat {
    count: usize,
    examples: Vec<String>,
}

#[derive(Default)]
struct KeyStat {
    /// The parent key of a nested `parent.child` key.
    parent: Option<String>,
    present: usize,
    forms: BTreeMap<&'static str, FormStat>,
    patterns: BTreeSet<String>,
}

/// All objects of one resource.
#[derive(Default)]
pub(crate) struct Survey {
    pub(crate) objects: usize,
    keys: BTreeMap<String, KeyStat>,
    /// How many objects carry each parent key as a nested object.
    parents: BTreeMap<String, usize>,
}

impl Survey {
    pub(crate) fn key_count(&self) -> usize {
        self.keys.len()
    }

    pub(crate) fn add_object(&mut self, object: &Map<String, Value>) {
        self.objects += 1;
        for (key, value) in object {
            self.record(key, None, value);
            if let Value::Object(inner) = value {
                *self.parents.entry(key.clone()).or_default() += 1;
                for (child, v) in inner {
                    self.record(&format!("{key}.{child}"), Some(key), v);
                }
            }
        }
    }

    fn record(&mut self, key: &str, parent: Option<&str>, value: &Value) {
        let stat = self.keys.entry(key.to_owned()).or_default();
        stat.parent = parent.map(str::to_owned);
        stat.present += 1;
        let (form, pattern, example) = classify(value);
        if let Some(p) = pattern {
            stat.patterns.insert(p);
        }
        let form_stat = stat.forms.entry(form).or_default();
        form_stat.count += 1;
        let recorded = if is_scalar(form) && is_sensitive(key) {
            None
        } else {
            Some(example)
        };
        if let Some(e) = recorded
            && form_stat.examples.len() < MAX_EXAMPLES
            && !form_stat.examples.contains(&e)
        {
            form_stat.examples.push(e);
        }
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out = String::from("key\tobjects");
        for f in FORMS {
            out.push('\t');
            out.push_str(f);
        }
        out.push_str("\tmissing\ttimestamp_patterns\texamples\n");
        for (key, stat) in &self.keys {
            let base = match &stat.parent {
                Some(p) => self.parents.get(p).copied().unwrap_or_default(),
                None => self.objects,
            };
            out.push_str(&format!("{}\t{}", tsv_cell(key), stat.present));
            for f in FORMS {
                out.push_str(&format!("\t{}", stat.forms.get(f).map_or(0, |s| s.count)));
            }
            let patterns: Vec<&str> = stat.patterns.iter().map(String::as_str).collect();
            let examples: Vec<String> = FORMS
                .iter()
                .filter_map(|f| {
                    let s = stat.forms.get(f)?;
                    (!s.examples.is_empty()).then(|| format!("{f}: {}", s.examples.join(" | ")))
                })
                .collect();
            out.push_str(&format!(
                "\t{}\t{}\t{}\n",
                base.saturating_sub(stat.present),
                tsv_cell(&patterns.join(", ")),
                tsv_cell(&examples.join("; "))
            ));
        }
        out
    }
}

fn tsv_cell(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

fn is_scalar(form: &str) -> bool {
    form != "object" && form != "array"
}

fn is_sensitive(key: &str) -> bool {
    let lower = key.to_lowercase();
    SENSITIVE.iter().any(|s| lower.contains(s))
}

/// The form column, an optional date pattern, and the example text of a value.
fn classify(value: &Value) -> (&'static str, Option<String>, String) {
    match value {
        Value::Null => ("null", None, "null".into()),
        Value::Bool(b) => ("bool", None, b.to_string()),
        Value::Number(n) => (
            if n.is_f64() { "float" } else { "int" },
            None,
            n.to_string(),
        ),
        Value::String(s) => classify_str(s),
        Value::Object(m) => {
            let mut keys: Vec<&str> = m.keys().map(String::as_str).collect();
            keys.sort_unstable();
            ("object", None, format!("object{{{}}}", keys.join(",")))
        }
        Value::Array(items) => {
            let forms: BTreeSet<&str> = items.iter().map(|v| classify(v).0).collect();
            let of = if forms.is_empty() {
                String::new()
            } else {
                format!(" of {}", forms.into_iter().collect::<Vec<_>>().join("|"))
            };
            ("array", None, format!("array[{}]{of}", items.len()))
        }
    }
}

fn classify_str(s: &str) -> (&'static str, Option<String>, String) {
    let shown = {
        let cut: String = s.chars().take(EXAMPLE_LEN).collect();
        Value::String(cut).to_string()
    };
    if s.is_empty() {
        return ("str_empty", None, shown);
    }
    if s == "true" || s == "false" {
        return ("str_bool_word", None, shown);
    }
    let unsigned = s.strip_prefix('-').unwrap_or(s);
    if all_digits(unsigned) {
        return ("str_int", None, shown);
    }
    if let Some((int, frac)) = unsigned.split_once('.')
        && all_digits(int)
        && all_digits(frac)
    {
        return ("str_decimal", None, shown);
    }
    match date_pattern(s) {
        Some((label, true)) => ("str_timestamp", Some(label), shown),
        Some((label, false)) => ("str_other", Some(format!("{label} (no time)")), shown),
        None => ("str_other", None, shown),
    }
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// The pattern label of a date-like string and whether it has a time part.
fn date_pattern(s: &str) -> Option<(String, bool)> {
    let mask: String = s
        .chars()
        .map(|c| if c.is_ascii_digit() { 'D' } else { c })
        .collect();
    let (date, mut rest) = if let Some(r) = mask.strip_prefix("DDDD-DD-DD") {
        ("YYYY-MM-DD", r)
    } else if let Some(r) = mask.strip_prefix("DD.DD.DDDD") {
        ("DD.MM.YYYY", r)
    } else {
        ("YYYY/MM/DD", mask.strip_prefix("DDDD/DD/DD")?)
    };
    let mut label = date.to_owned();
    let mut has_time = false;
    if let Some(sep) = rest.chars().next().filter(|c| matches!(c, 'T' | ' '))
        && let Some(r) = rest[1..].strip_prefix("DD:DD:DD")
    {
        label.push(sep);
        label.push_str("HH:MM:SS");
        rest = r;
        has_time = true;
    }
    if has_time {
        if let Some(r) = rest.strip_prefix('.') {
            let n = r.bytes().take_while(|b| *b == b'D').count();
            if n > 0 {
                label.push_str(&format!(".<{n} digits>"));
                rest = &r[n..];
            }
        }
        if let Some(r) = rest.strip_prefix('Z') {
            label.push('Z');
            rest = r;
        } else if let Some(r) = rest.strip_prefix(['+', '-']) {
            for (digits, tz) in [("DD:DD", "±HH:MM"), ("DDDD", "±HHMM"), ("DD", "±HH")] {
                if let Some(r2) = r.strip_prefix(digits) {
                    label.push_str(tz);
                    rest = r2;
                    break;
                }
            }
        }
    }
    if !rest.is_empty() {
        label.push_str(&format!(" +trailing {rest}"));
    }
    Some((label, has_time))
}
