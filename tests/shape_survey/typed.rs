//! `typed_all`: the release gate. Every live object of the test account is
//! decoded through the crate's PUBLIC read types; every decode error (with its
//! field path) and every key the type does not read is reported. Read-only
//! like the rest of the target (GET only, account check first). The RAW answer
//! text is decoded, never the collapsed one, so the real shapes are tested;
//! only key paths, error kinds and counts are printed or written, never a value.
//!
//! The crate's drift guard (`wire::Guard`) is `pub(crate)` and not reachable
//! from an integration test: it is not run here.

use super::harness::{Ctx, SERIAL};
use super::{MAX_PAGES, PAGE, RESOURCES, get, verify_test_account};
use serde::Deserialize;
use serde_json::value::RawValue;
use sevdesk::{
    CheckAccount, CommunicationWay, Contact, ContactAddress, CreditNote, CreditNotePosition,
    Invoice, InvoicePosition, PaymentMethod, SevSequence, SevUser, Transaction,
    TransactionLogEntry, Voucher, VoucherPosition,
};
use std::collections::BTreeMap;

/// Decodes the `objects` of one answer into `Acc` and returns how many objects it held.
type Decoder = fn(&mut Acc, &str) -> usize;

const TYPED: &[(&str, Decoder)] = &[
    ("Invoice", decode_answer::<Invoice>),
    ("InvoicePos", decode_answer::<InvoicePosition>),
    ("CreditNote", decode_answer::<CreditNote>),
    ("CreditNotePos", decode_answer::<CreditNotePosition>),
    ("Voucher", decode_answer::<Voucher>),
    ("VoucherPos", decode_answer::<VoucherPosition>),
    ("CheckAccountTransaction", decode_answer::<Transaction>),
    (
        "CheckAccountTransactionLog",
        decode_answer::<TransactionLogEntry>,
    ),
    ("CheckAccount", decode_answer::<CheckAccount>),
    ("Contact", decode_answer::<Contact>),
    ("ContactAddress", decode_answer::<ContactAddress>),
    ("CommunicationWay", decode_answer::<CommunicationWay>),
    ("SevUser", decode_answer::<SevUser>),
    ("PaymentMethod", decode_answer::<PaymentMethod>),
    ("SevSequence", decode_answer::<SevSequence>),
];

/// Findings of one resource: nothing in here is a value.
#[derive(Default)]
struct Acc {
    objects: usize,
    /// (field path, error kind) -> (count, id of the first object)
    errors: BTreeMap<(String, String), (usize, String)>,
    /// normalized unread key -> count
    unread: BTreeMap<String, usize>,
}

impl Acc {
    fn error_count(&self) -> usize {
        self.errors.values().map(|(n, _)| n).sum()
    }

    fn to_tsv(&self) -> String {
        let mut out = String::from("what\tpath_or_key\terror_kind\tcount\tfirst_object_id\n");
        for ((path, kind), (n, id)) in &self.errors {
            out.push_str(&format!("error\t{path}\t{kind}\t{n}\t{id}\n"));
        }
        for (key, n) in &self.unread {
            out.push_str(&format!("unread\t{key}\t\t{n}\t\n"));
        }
        out
    }
}

/// Replaces every list index (`.0.`, `[0]`) by `*`.
fn normalize_path(path: &str) -> String {
    let mut out = String::new();
    let mut digits = String::new();
    let flush = |digits: &mut String, out: &mut String| {
        if !digits.is_empty() {
            out.push('*');
            digits.clear();
        }
    };
    let mut prev_boundary = true;
    for c in path.chars() {
        if c.is_ascii_digit() && prev_boundary {
            digits.push(c);
            continue;
        }
        if !digits.is_empty() {
            if c == '.' || c == ']' {
                flush(&mut digits, &mut out);
            } else {
                out.push_str(&digits);
                digits.clear();
            }
        }
        out.push(c);
        prev_boundary = c == '.' || c == '[';
    }
    flush(&mut digits, &mut out);
    out.replace("?", "*")
}

/// Cuts a serde message down to a value-free kind: quoted parts (`"…"`, `` `…` ``)
/// are dropped except in `missing field`, digits are masked, the length is capped.
fn error_kind(message: &str) -> String {
    let mut kind = if message.starts_with("missing field") {
        message.to_owned()
    } else {
        let mut out = String::new();
        let mut quote: Option<char> = None;
        for c in message.chars() {
            match (quote, c) {
                (None, '"' | '`') => {
                    quote = Some(c);
                    out.push_str("<..>");
                }
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, c) => out.push(if c.is_ascii_digit() { '#' } else { c }),
            }
        }
        out
    };
    kind = kind.replace(['\t', '\n', '\r'], " ");
    kind.chars().take(80).collect()
}

/// The id of an object when it is plain digits, else `?`.
fn object_id(raw: &RawValue) -> String {
    serde_json::from_str::<serde_json::Value>(raw.get())
        .ok()
        .and_then(|v| match &v["id"] {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .unwrap_or_else(|| "?".into())
}

/// Decodes one object into `T`, recording decode errors and unread keys.
fn decode_object<T: for<'de> Deserialize<'de>>(acc: &mut Acc, raw: &RawValue) {
    let mut de = serde_json::Deserializer::from_str(raw.get());
    let mut unread = Vec::new();
    let result = serde_path_to_error::deserialize::<_, T>(serde_ignored::Deserializer::new(
        &mut de,
        &mut |p: serde_ignored::Path<'_>| unread.push(normalize_path(&p.to_string())),
    ));
    if let Err(e) = result {
        let path = normalize_path(&e.path().to_string());
        let kind = error_kind(&e.inner().to_string());
        let entry = acc
            .errors
            .entry((path, kind))
            .or_insert_with(|| (0, object_id(raw)));
        entry.0 += 1;
    }
    for key in unread {
        *acc.unread.entry(format!("objects.*.{key}")).or_default() += 1;
    }
}

/// `objects` of an answer: a list, or one bare object (`SevSequence`).
fn decode_answer<T: for<'de> Deserialize<'de>>(acc: &mut Acc, text: &str) -> usize {
    #[derive(Deserialize)]
    struct Envelope {
        objects: Box<RawValue>,
    }
    let Ok(envelope) = serde_json::from_str::<Envelope>(text) else {
        eprintln!("TYPED-NOTE\tanswer has no `objects`");
        return 0;
    };
    let raw = envelope.objects;
    let list: Vec<Box<RawValue>> = if raw.get().trim_start().starts_with('[') {
        match serde_json::from_str(raw.get()) {
            Ok(l) => l,
            Err(_) => {
                eprintln!("TYPED-NOTE\t`objects` is not a list of values");
                return 0;
            }
        }
    } else {
        vec![raw]
    };
    for item in &list {
        acc.objects += 1;
        decode_object::<T>(acc, item);
    }
    list.len()
}

async fn typed_resource(ctx: &Ctx, name: &str, decode: Decoder) -> (usize, usize) {
    let res = RESOURCES.iter().find(|r| r.name == name).unwrap();
    let mut acc = Acc::default();
    'paths: for path in res.paths {
        for page in 0..if res.paged { MAX_PAGES } else { 1 } {
            let url = if res.paged {
                format!("{path}?limit={PAGE}&offset={}", page * PAGE)
            } else {
                (*path).to_owned()
            };
            let (status, text) = get(ctx, &url).await;
            if status >= 300 {
                eprintln!("TYPED-NOTE\t{name}\t{url}\t{status}\tskipped");
                break 'paths;
            }
            let n = decode(&mut acc, &text);
            if n < PAGE {
                break;
            }
            if page + 1 == MAX_PAGES {
                eprintln!("TYPED-NOTE\t{name}\tcapped at {MAX_PAGES} pages");
            }
        }
    }
    let dir = format!("{}/shape_survey", env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/typed_{name}.tsv"), acc.to_tsv()).unwrap();
    for ((path, kind), (n, id)) in &acc.errors {
        eprintln!("TYPED-ERROR\t{name}\t{path}\t{kind}\t{n}\tfirst id {id}");
    }
    for (key, n) in &acc.unread {
        eprintln!("TYPED-UNREAD\t{name}\t{key}\t{n}");
    }
    eprintln!(
        "TYPED\t{name}\t{}\t{}\t{}",
        acc.objects,
        acc.error_count(),
        acc.unread.len()
    );
    (acc.objects, acc.error_count())
}

/// The release gate: a measurement, so it passes with findings and prints them.
#[tokio::test]
#[ignore = "reads the whole sevDesk test account"]
async fn typed_all() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping typed_all");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ctx = Ctx::new(&token, "shape-survey".into());
    verify_test_account(&ctx).await;
    let (mut objects, mut errors) = (0, 0);
    for (name, decode) in TYPED {
        let (o, e) = typed_resource(&ctx, name, *decode).await;
        objects += o;
        errors += e;
    }
    eprintln!("TYPED-TOTAL\t{objects}\t{errors}");
}
