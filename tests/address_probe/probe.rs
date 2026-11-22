//! The case: four drafts, three reads each, compared with what was sent.
use crate::build::{Env, Kind};
use crate::docs::raw_objects;
use crate::harness::*;
use crate::record::Outcome;
use crate::scrub::scrub_capture;
use reqwest::Method;
use serde_json::Value;
use sevdesk::*;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use time::OffsetDateTime;

const HEADER: &str = "step\tkind\tid\tread\thttp\tname\tstreet\tzip\tcity\tcountry_id\taddress\tall_equal\tdiffering_fields";

/// The compared fields, in column order; `address` is only recorded.
const FIELDS: [&str; 5] = ["name", "street", "zip", "city", "country_id"];

/// Five address values (name, street, zip, city, country id) and the
/// free-text `address`, each as JSON (`null` when absent).
struct Seen {
    fields: [Value; 5],
    address: Value,
}

fn text(v: Option<&String>) -> Value {
    v.map_or(Value::Null, |s| Value::String(s.clone()))
}

/// An id that sevDesk sends as a string or a number, as a string.
fn id_text(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(s.clone()),
        Value::Number(n) => Value::String(n.to_string()),
        _ => Value::Null,
    }
}

impl Seen {
    fn from_object(o: &Value) -> Self {
        let s = |k: &str| {
            o[k].as_str()
                .map_or(Value::Null, |s| Value::String(s.into()))
        };
        Self {
            fields: [
                s("addressName"),
                s("addressStreet"),
                s("addressZip"),
                s("addressCity"),
                id_text(&o["addressCountry"]["id"]),
            ],
            address: o["address"].clone(),
        }
    }

    fn from_invoice(i: &Invoice) -> Self {
        Self {
            fields: [
                text(i.address_name.as_ref()),
                text(i.address_street.as_ref()),
                text(i.address_zip.as_ref()),
                text(i.address_city.as_ref()),
                i.address_country
                    .map_or(Value::Null, |c| Value::String(c.id.get().to_string())),
            ],
            address: Value::String("(not in the typed save answer)".into()),
        }
    }

    fn from_credit_note(n: &CreditNote) -> Self {
        Self {
            fields: [
                text(n.address_name.as_ref()),
                text(n.address_street.as_ref()),
                text(n.address_zip.as_ref()),
                text(n.address_city.as_ref()),
                n.address_country
                    .map_or(Value::Null, |c| Value::String(c.id.get().to_string())),
            ],
            address: Value::String("(not in the typed save answer)".into()),
        }
    }
}

struct Probe {
    env: Env,
    tsv: File,
    _alone: tokio::sync::MutexGuard<'static, ()>,
}

impl Probe {
    /// `None` without a token. `Env::new` aborts on any account but the test
    /// account, before the first write.
    async fn new(case: &str) -> Option<Self> {
        let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
            eprintln!("SEVDESK_API_KEY is not set: skipping the address probe");
            return None;
        };
        let alone = SERIAL.lock().await;
        let ts = OffsetDateTime::now_utc().unix_timestamp();
        let ctx = Arc::new(Ctx::new(&token, format!("SDRS-ADR-{ts}")));
        let env = Env::new(&format!("adr-{case}"), ctx).await;
        let dir = format!("{}/address_probe", env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut tsv = File::create(format!("{dir}/{case}.tsv")).unwrap();
        writeln!(tsv, "{HEADER}").unwrap();
        Some(Self {
            env,
            tsv,
            _alone: alone,
        })
    }

    fn line(&mut self, cols: &[String]) {
        let line = cols.join("\t");
        eprintln!("ADDRESS\t{line}");
        writeln!(self.tsv, "{line}").unwrap();
    }

    /// One read compared with `sent` (exact equality, no trimming).
    fn record(
        &mut self,
        (step, kind, id): (&str, Kind, i64),
        read: &str,
        http: &str,
        sent: &[Value; 5],
        seen: &Seen,
    ) {
        let differing: Vec<&str> = FIELDS
            .iter()
            .zip(sent.iter().zip(&seen.fields))
            .filter(|(_, (a, b))| a != b)
            .map(|(f, _)| *f)
            .collect();
        let json = |v: &Value| serde_json::to_string(v).unwrap();
        let mut cols = vec![
            step.to_owned(),
            kind.obj().to_owned(),
            id.to_string(),
            read.to_owned(),
            http.to_owned(),
        ];
        cols.extend(seen.fields.iter().map(json));
        cols.push(json(&seen.address));
        cols.push(differing.is_empty().to_string());
        cols.push(if differing.is_empty() {
            "-".to_owned()
        } else {
            differing.join(",")
        });
        self.line(&cols);
    }

    /// A raw GET that must answer below 300; panics with the scrubbed body.
    async fn get(&self, path: &str) -> (u16, Value) {
        let (status, body) = self.env.ctx.raw(Method::GET, path, None, None).await;
        assert!(
            status < 300,
            "GET {path} answered {status}: {}",
            scrub_capture(&body.chars().take(2000).collect::<String>())
        );
        (status, raw_objects(&body))
    }

    /// The `GET /<kind>/{id}` read and the list read filtered by `note`.
    async fn reread(&mut self, what: (&str, Kind, i64), note: &str, sent: &[Value; 5]) {
        let (step, kind, id) = what;
        let obj = kind.obj();
        let (status, got) = self.get(&format!("/{obj}/{id}")).await;
        let one = match &got {
            Value::Array(l) => l.first().cloned().unwrap_or(Value::Null),
            other => other.clone(),
        };
        assert!(one.is_object(), "GET /{obj}/{id} returned no object");
        self.record(
            what,
            "get",
            &status.to_string(),
            sent,
            &Seen::from_object(&one),
        );

        let (status, listed) = self
            .get(&format!("/{obj}?customerInternalNote={note}"))
            .await;
        let list = listed.as_array().cloned().unwrap_or_default();
        let found = list
            .iter()
            .find(|o| id_text(&o["id"]) == Value::String(id.to_string()))
            .unwrap_or_else(|| panic!("{step}: list by note {note} has no {obj} {id}"));
        self.record(
            what,
            "list",
            &status.to_string(),
            sent,
            &Seen::from_object(found),
        );
    }

    /// A failed save: the error, then the half-written check by note.
    async fn failed(&mut self, step: &str, kind: Kind, note: &str, out: &Outcome) {
        let half = self.half_written(kind, note).await;
        let dash = || "-".to_owned();
        let cols = [
            step.to_owned(),
            kind.obj().to_owned(),
            dash(),
            "save".to_owned(),
            out.status.to_string(),
            dash(),
            dash(),
            dash(),
            dash(),
            dash(),
            dash(),
            dash(),
            format!(
                "save failed: code={:?} {}; half_written: {half}",
                out.code, out.message
            ),
        ];
        self.line(&cols);
    }

    /// Looks for a document a failed save may have left behind.
    async fn half_written(&self, kind: Kind, note: &str) -> String {
        let c = &self.env.ctx.c;
        let found: Result<Vec<String>, String> = match kind {
            Kind::Invoice => c
                .list_invoices(&InvoiceQuery {
                    customer_internal_note: Some(note.to_owned()),
                    ..Default::default()
                })
                .await
                .map(|l| {
                    l.iter()
                        .map(|i| format!("Invoice {}", i.id.get()))
                        .collect()
                })
                .map_err(|e| e.to_string()),
            Kind::CreditNote => c
                .list_credit_notes(&CreditNoteQuery {
                    customer_internal_note: Some(note.to_owned()),
                    ..Default::default()
                })
                .await
                .map(|l| {
                    l.iter()
                        .map(|n| format!("CreditNote {}", n.id.get()))
                        .collect()
                })
                .map_err(|e| e.to_string()),
        };
        match found {
            Ok(v) => format!("found {}: {}", v.len(), v.join("; ")),
            Err(e) => format!("list failed: {e}"),
        }
    }

    /// Saves one draft with all five address fields set and rereads it.
    async fn draft(&mut self, step: &str, kind: Kind, name: String) {
        let note = format!("{}-{step}", self.env.ctx.marker);
        let street = "Messweg 7".to_owned();
        let zip = "10115".to_owned();
        let city = "Berlin".to_owned();
        let country = ObjectRef::new(CountryId::new(1));
        let sent: [Value; 5] = [
            Value::String(name.clone()),
            Value::String(street.clone()),
            Value::String(zip.clone()),
            Value::String(city.clone()),
            Value::String("1".into()),
        ];
        let saved = match kind {
            Kind::Invoice => {
                let mut save = self.env.invoice_save(note.clone(), 1);
                let d = &mut save.invoice.options;
                d.address_name = Some(name);
                d.address_street = Some(street);
                d.address_zip = Some(zip);
                d.address_city = Some(city);
                d.address_country = Some(country);
                match self.env.ctx.c.save_invoice(&save).await {
                    Ok(s) => Ok((s.invoice.id.get(), Seen::from_invoice(&s.invoice))),
                    Err(e) => Err(Outcome::from_error(&e)),
                }
            }
            Kind::CreditNote => {
                let number = self.env.ctx.c.next_credit_note_number().await.unwrap();
                let mut save = self
                    .env
                    .credit_note_save(number, note.clone(), self.env.origin);
                let d = &mut save.credit_note.options;
                d.address_name = Some(name);
                d.address_street = Some(street);
                d.address_zip = Some(zip);
                d.address_city = Some(city);
                d.address_country = Some(country);
                match self.env.ctx.c.save_credit_note(&save).await {
                    Ok(s) => Ok((
                        s.credit_note.id.get(),
                        Seen::from_credit_note(&s.credit_note),
                    )),
                    Err(e) => Err(Outcome::from_error(&e)),
                }
            }
        };
        match saved {
            Ok((id, seen)) => {
                self.record((step, kind, id), "save", "ok", &sent, &seen);
                self.reread((step, kind, id), &note, &sent).await;
            }
            Err(out) => self.failed(step, kind, &note, &out).await,
        }
    }
}

/// Case 1: invoice and credit note, each with a plain and a space-padded
/// `addressName`.
pub(crate) async fn case1(case: &str) {
    let Some(mut p) = Probe::new(case).await else {
        return;
    };
    let marker = p.env.ctx.marker.clone();
    let plain = format!("{marker} Adresse GmbH");
    let spaces = format!("  {marker} Adresse GmbH  ");
    p.draft("invoice_plain", Kind::Invoice, plain.clone()).await;
    p.draft("credit_note_plain", Kind::CreditNote, plain).await;
    p.draft("invoice_spaces", Kind::Invoice, spaces.clone())
        .await;
    p.draft("credit_note_spaces", Kind::CreditNote, spaces)
        .await;
}
