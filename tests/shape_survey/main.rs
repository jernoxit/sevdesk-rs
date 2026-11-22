//! The shape survey: which value FORMS occur per resource and JSON key in the
//! sevDesk TEST account. It feeds the decision which tolerant readers the crate
//! needs. Strictly READ-ONLY: every request is a `GET` (`only_get` asserts it),
//! and before the first list call `GET /SevClient` must name the test account
//! (id 1303847), otherwise the run aborts. It is its own test target
//! (`test = false` in Cargo.toml) and runs only on request:
//!
//! `just test-live --test shape_survey survey_all`
//!
//! Resources are listed with `limit=1000&offset=N` until a page has fewer than
//! 1000 objects, at most 10 pages. A resource that answers 300 or more is
//! recorded as such and skipped. Output: one TSV per resource in
//! `target/tmp/shape_survey/<Resource>.tsv` and one stderr line per resource,
//! `SURVEY\t<Resource>\t<http>\t<objects>\t<keys>`. `survey_recordings` is the
//! offline twin: the same classifier over `tests/fixtures/*.json` and
//! `target/tmp/captures/*.json`, grouped by `objectName` into
//! `recorded_<objectName>.tsv` (no network, no token). Expanded `SevClient` /
//! `SevUser` objects are collapsed by the shared scrubber before they are
//! classified (except the `SevUser` list itself: keys and forms, no examples); keys that may hold personal or secret data record no example
//! values. `survey_all` also writes up to 4 scrubbed sample objects per
//! resource to `target/tmp/shape_survey/samples/<Resource>.json` plus a
//! `.coverage.tsv` sidecar (none for `SevUser`); see `sample.rs`. `typed_all` (see `typed.rs`) is the release gate: it
//! decodes the raw answers through the crate's public read types. `filters_all`
//! (see `filters.rs`) measures every list filter the spec names, read-only.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness file is shared with live_write and uses only some of its helpers; a failure in a test helper is a test failure"
)]

mod filter_cases;
mod filter_judge;
mod filter_table;
mod filters;
mod filters_followup;
mod followup_hyp;
mod followup_more;
#[path = "../live_write/harness.rs"]
mod harness;
mod sample;
#[path = "../common/scrub.rs"]
mod scrub;
mod shape;
mod typed;

use harness::{Ctx, SERIAL};
use reqwest::Method;
use serde_json::Value;
use shape::Survey;

const TEST_SEV_CLIENT: i64 = 1303847;
const PAGE: usize = 1000;
const MAX_PAGES: usize = 10;

struct Resource {
    name: &'static str,
    paths: &'static [&'static str],
    paged: bool,
}

const fn paged(name: &'static str, path: &'static [&'static str]) -> Resource {
    Resource {
        name,
        paths: path,
        paged: true,
    }
}

const RESOURCES: &[Resource] = &[
    paged("Invoice", &["/Invoice"]),
    paged("InvoicePos", &["/InvoicePos"]),
    paged("CreditNote", &["/CreditNote"]),
    paged("CreditNotePos", &["/CreditNotePos"]),
    paged("Voucher", &["/Voucher"]),
    paged("VoucherPos", &["/VoucherPos"]),
    paged("CheckAccountTransaction", &["/CheckAccountTransaction"]),
    paged(
        "CheckAccountTransactionLog",
        &["/CheckAccountTransactionLog"],
    ),
    paged("CheckAccount", &["/CheckAccount"]),
    paged("Contact", &["/Contact"]),
    paged("ContactAddress", &["/ContactAddress"]),
    paged("CommunicationWay", &["/CommunicationWay"]),
    paged("SevUser", &["/SevUser"]),
    paged("PaymentMethod", &["/PaymentMethod"]),
    paged("StaticCountry", &["/StaticCountry"]),
    Resource {
        name: "SevSequence",
        paths: &[
            "/SevSequence/Factory/getByType?objectType=Invoice&type=RE",
            "/SevSequence/Factory/getByType?objectType=CreditNote&type=CN",
        ],
        paged: false,
    },
];

/// The only method this target ever sends.
fn only_get(method: Method) -> Method {
    assert_eq!(method, Method::GET, "the shape survey is read-only");
    method
}

async fn get(ctx: &Ctx, path: &str) -> (u16, String) {
    ctx.raw(only_get(Method::GET), path, None, None).await
}

/// `objects` of an answer as a list, expanded account objects collapsed
/// unless `collapse` is off (the `SevUser` list itself).
fn objects_of(text: &str, collapse: bool) -> Option<Vec<Value>> {
    let mut v: Value = serde_json::from_str(text).ok()?;
    if collapse {
        scrub::collapse_account_objects(&mut v);
    }
    match v.get_mut("objects")?.take() {
        Value::Array(list) => Some(list),
        obj @ Value::Object(_) => Some(vec![obj]),
        _ => None,
    }
}

/// Aborts unless `GET /SevClient` names the test account.
async fn verify_test_account(ctx: &Ctx) {
    let (status, text) = get(ctx, "/SevClient").await;
    assert!(status < 300, "GET /SevClient answered {status}");
    let id = objects_of(&text, true)
        .and_then(|l| l.first().map(|o| o["id"].clone()))
        .and_then(|id| match id {
            Value::String(s) => s.parse::<i64>().ok(),
            Value::Number(n) => n.as_i64(),
            _ => None,
        });
    assert_eq!(
        id,
        Some(TEST_SEV_CLIENT),
        "not the test account: aborting before the survey"
    );
}

async fn survey_resource(ctx: &Ctx, res: &Resource) {
    // The `SevUser` list is the one place account objects are surveyed as
    // they are: keys and forms, never a value.
    let own_users = res.name == "SevUser";
    let mut survey = if own_users {
        Survey::without_examples()
    } else {
        Survey::default()
    };
    let mut pool: Vec<serde_json::Map<String, Value>> = Vec::new();
    let mut http = 0;
    'paths: for path in res.paths {
        for page in 0..if res.paged { MAX_PAGES } else { 1 } {
            let url = if res.paged {
                format!("{path}?limit={PAGE}&offset={}", page * PAGE)
            } else {
                (*path).to_owned()
            };
            let (status, text) = get(ctx, &url).await;
            http = http.max(status);
            if status >= 300 {
                eprintln!("SURVEY-NOTE\t{}\t{url}\t{status}\tskipped", res.name);
                break 'paths;
            }
            let Some(list) = objects_of(&text, !own_users) else {
                eprintln!("SURVEY-NOTE\t{}\t{url}\tno objects array", res.name);
                break;
            };
            let n = list.len();
            for o in &list {
                match o {
                    Value::Object(m) => {
                        survey.add_object(m);
                        pool.push(m.clone());
                    }
                    other => eprintln!("SURVEY-NOTE\t{}\tnon-object element: {other:.0}", res.name),
                }
            }
            if n < PAGE {
                break;
            }
            if page + 1 == MAX_PAGES {
                eprintln!("SURVEY-NOTE\t{}\tcapped at {MAX_PAGES} pages", res.name);
            }
        }
    }
    let dir = format!("{}/shape_survey", env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/{}.tsv", res.name), survey.to_tsv()).unwrap();
    if !own_users {
        sample::write_samples(&dir, res.name, &pool);
    }
    eprintln!(
        "SURVEY\t{}\t{http}\t{}\t{}",
        res.name,
        survey.objects,
        survey.key_count()
    );
}

/// Every object carrying an `objectName`, at any depth, grouped by that name.
fn collect_objects(v: &Value, file: &str, out: &mut std::collections::BTreeMap<String, Survey>) {
    match v {
        Value::Object(m) => {
            if let Some(Value::String(name)) = m.get("objectName") {
                let survey = out.entry(name.clone()).or_insert_with(Survey::with_files);
                survey.set_source(file);
                survey.add_object(m);
            }
            m.values().for_each(|x| collect_objects(x, file, out));
        }
        Value::Array(list) => list.iter().for_each(|x| collect_objects(x, file, out)),
        _ => {}
    }
}

/// Offline: the same classifier over recorded answers. No network, no token.
fn survey_recorded_files() {
    let root = env!("CARGO_MANIFEST_DIR");
    let tmp = env!("CARGO_TARGET_TMPDIR");
    let mut surveys = std::collections::BTreeMap::new();
    for dir in [format!("{root}/tests/fixtures"), format!("{tmp}/captures")] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for path in paths {
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(mut v) = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            else {
                continue;
            };
            scrub::collapse_account_objects(&mut v);
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            collect_objects(&v, &file, &mut surveys);
        }
    }
    let out = format!("{tmp}/shape_survey");
    std::fs::create_dir_all(&out).unwrap();
    for (name, survey) in &surveys {
        std::fs::write(format!("{out}/recorded_{name}.tsv"), survey.to_tsv()).unwrap();
        eprintln!(
            "SURVEY-RECORDED\t{name}\t{}\t{}",
            survey.objects,
            survey.key_count()
        );
    }
}

#[test]
#[ignore = "run with the shape survey"]
fn survey_recordings() {
    survey_recorded_files();
}

async fn run(names: &[&str]) {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping shape survey");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ctx = Ctx::new(&token, "shape-survey".into());
    verify_test_account(&ctx).await;
    for res in RESOURCES.iter().filter(|r| names.contains(&r.name)) {
        survey_resource(&ctx, res).await;
    }
}

#[tokio::test]
#[ignore = "reads the whole sevDesk test account"]
async fn survey_all() {
    let names: Vec<&str> = RESOURCES.iter().map(|r| r.name).collect();
    run(&names).await;
}

macro_rules! one {
    ($test:ident, $name:literal) => {
        #[tokio::test]
        #[ignore = "reads one resource of the sevDesk test account"]
        async fn $test() {
            run(&[$name]).await;
        }
    };
}

one!(survey_invoice, "Invoice");
one!(survey_invoice_pos, "InvoicePos");
one!(survey_credit_note, "CreditNote");
one!(survey_credit_note_pos, "CreditNotePos");
one!(survey_voucher, "Voucher");
one!(survey_voucher_pos, "VoucherPos");
one!(survey_transaction, "CheckAccountTransaction");
one!(survey_transaction_log, "CheckAccountTransactionLog");
one!(survey_check_account, "CheckAccount");
one!(survey_contact, "Contact");
one!(survey_contact_address, "ContactAddress");
one!(survey_communication_way, "CommunicationWay");
one!(survey_sev_user, "SevUser");
one!(survey_payment_method, "PaymentMethod");
one!(survey_static_country, "StaticCountry");
one!(survey_sev_sequence, "SevSequence");
