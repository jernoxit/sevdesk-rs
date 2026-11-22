//! The filter setup probe: CREATES the data the test account lacks and then
//! measures the list filters that were unclear or unmeasurable on the account
//! as it was (tags, cost centres, the voucher supplier, recurring invoices,
//! position comments, `delinquent`, `noRv`, `withoutCatering`, `day`). Writes
//! PERMANENT objects to the sevDesk TEST account (the probe aborts on any other
//! account before the first write), so it is its own test target
//! (`test = false` in Cargo.toml) and runs only on request:
//!
//! `just test-live --test filter_setup_probe case1`
//!
//! A filter is judged as in the read-only follow-ups: the filtered answer is
//! compared with the set a stated predicate selects from the unfiltered list.
//!
//! Setup (routes from the OpenAPI spec): `POST /Tag/Factory/create` (name and
//! the object it tags; ONE tag, created on the new voucher), `POST /TagRelation`
//! (spec: only `GET`; tried once per invoice, credit note and contact), cost
//! centre: the spec names NO route, so `GET /CostCentre` is read first and, when
//! empty, `POST /CostCentre` is tried once. Cost centres go onto the existing
//! probe invoice and credit note drafts through `PUT /Invoice/{id}` and
//! `PUT /CreditNote/{id}`, onto the new voucher through the save factory with
//! its id. One new voucher draft (supplier contact, a position comment) and one
//! invoice draft with `invoiceType` WKR are the only new documents.
//!
//! Budget: at most 7 permanent objects (voucher, tag, cost centre, WKR invoice,
//! up to 3 tag relations) and no consumed document number. Every object prints
//! `CREATED <kind> <id>`; the object counter aborts before the 8th. No write is
//! retried. Output: `target/tmp/filter_setup_probe/results.tsv` and stderr
//! `FILTER2\t<endpoint>\t<filter>\t<form>\t<class>\t<filtered/unfiltered>\t<note>`.
//! No value of the account is printed.
//!
//! Left alone as "needs a finalized document": `showMa`, `onlyDunned`,
//! `partiallyPaid`.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the harness files are shared with live_write and use only some of the helpers; a failure in a test helper is a test failure"
)]

#[path = "../live_write/docs.rs"]
mod docs;
#[path = "../live_write/harness.rs"]
mod harness;
mod measure;
#[path = "../common/scrub.rs"]
mod scrub;

use docs::{Customer, InvoiceSpec, contact_person, invoice_draft, net_position};
use harness::{Ctx, SERIAL, position, with_ids};
use measure::{
    Probe, Results, error_note, failed, first_object, hyp, list, pairs, ref_id, sid, text,
};
use reqwest::Method;
use serde_json::{Value, json};
use sevdesk::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

const TEST_SEV_CLIENT: i64 = 1303847;
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";
const BUDGET_OBJECTS: usize = 7;
const COMMENT: &str = "SDRS-FILTER2 position comment";
const COMMENT_PART: &str = "FILTER2 position";

static CREATED: AtomicUsize = AtomicUsize::new(0);

fn created(what: &str, id: impl std::fmt::Display) {
    CREATED.fetch_add(1, Ordering::SeqCst);
    eprintln!("CREATED {what} {id}");
}

/// Stops before the write that would exceed the budget.
fn may_create() {
    assert!(
        CREATED.load(Ordering::SeqCst) < BUDGET_OBJECTS,
        "object budget exhausted"
    );
}

/// Aborts unless `GET /SevClient` names the test account.
async fn verify_test_account(ctx: &Ctx) {
    let (status, body) = ctx.raw(Method::GET, "/SevClient", None, None).await;
    assert!(status < 300, "GET /SevClient answered {status}");
    let id = first_object(&body)
        .and_then(|o| sid(&o["id"]))
        .and_then(|s| s.parse::<i64>().ok());
    assert_eq!(
        id,
        Some(TEST_SEV_CLIENT),
        "not the test account: aborting before the write"
    );
}

async fn existing_customer(ctx: &Ctx) -> Customer {
    let found = ctx.c.list_contacts_by_name(CONTACT_PREFIX).await.unwrap();
    let contact = found
        .into_iter()
        .find(|c| {
            c.name
                .as_deref()
                .is_some_and(|n| n.starts_with(CONTACT_PREFIX))
        })
        .unwrap_or_else(|| panic!("no existing contact named `{CONTACT_PREFIX}...`: aborting"));
    let name = contact.name.clone().expect("the contact has a name");
    Customer {
        contact,
        name,
        street: docs::STREET,
        zip: docs::ZIP,
        city: docs::CITY,
    }
}

/// An existing probe document (marker `SDRS-` in `field`), a draft (status 100) first.
fn probe_doc(list: &[Value], field: &str) -> Option<Value> {
    let mut found: Vec<&Value> = list
        .iter()
        .filter(|o| text(o, field).is_some_and(|s| s.starts_with("SDRS-")))
        .collect();
    found.sort_by_key(|o| text(o, "status").as_deref() != Some("100"));
    found.first().map(|o| (*o).clone())
}

fn object_ref(id: i64, name: &str) -> Value {
    json!({"id": id, "objectName": name})
}

/// The new voucher draft: updates go through the save factory with its id.
struct VoucherSetup {
    id: i64,
    base: Value,
}

impl VoucherSetup {
    /// One factory update; the patch stays in `base` only when sevDesk took it.
    async fn update(&mut self, ctx: &Ctx, patch: &[(&str, Value)]) -> String {
        let mut body = self.base.clone();
        for (k, v) in patch {
            body["voucher"][*k] = v.clone();
        }
        let (status, answer) = ctx
            .raw(
                Method::POST,
                "/Voucher/Factory/saveVoucher",
                Some(body.to_string()),
                None,
            )
            .await;
        if failed(status, &answer) {
            return error_note(status, &answer);
        }
        let got = serde_json::from_str::<Value>(&answer)
            .ok()
            .and_then(|v| sid(&v["objects"]["voucher"]["id"]))
            .and_then(|s| s.parse::<i64>().ok());
        if let Some(got) = got.filter(|g| *g != self.id) {
            eprintln!("CREATED Voucher {got} (UNEXPECTED, by the factory update)");
            panic!("the factory created a new voucher instead of updating: stopping");
        }
        self.base = body;
        "ok".into()
    }
}

async fn write(ctx: &Ctx, method: Method, path: &str, body: Value) -> (u16, String) {
    ctx.raw(method, path, Some(body.to_string()), None).await
}

fn today() -> String {
    OffsetDateTime::now_utc().date().to_string()
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_clarify_filters() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the filter setup probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-FILTER2-{ts}"));
    verify_test_account(&ctx).await;
    let mut notes: BTreeMap<&'static str, String> = BTreeMap::new();
    let none = Vec::new();

    // Read-only inputs.
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    let contact_id = customer.contact.id.get();
    let invoices = list(&ctx, "/Invoice", &none).await.unwrap();
    let credit_notes = list(&ctx, "/CreditNote", &none).await.unwrap();
    let inv_doc = probe_doc(&invoices, "customerInternalNote");
    let cn_doc = probe_doc(&credit_notes, "customerInternalNote");
    let id_of = |o: &Option<Value>| -> Option<i64> {
        o.as_ref()
            .and_then(|o| sid(&o["id"]))
            .and_then(|s| s.parse().ok())
    };
    let (inv_id, cn_id) = (id_of(&inv_doc), id_of(&cn_doc));
    let is_draft = |o: &Option<Value>| {
        o.as_ref()
            .is_some_and(|o| text(o, "status").as_deref() == Some("100"))
    };
    eprintln!(
        "SETUP probe invoice {inv_id:?} (draft {}), probe credit note {cn_id:?} (draft {}), contact {contact_id}",
        is_draft(&inv_doc),
        is_draft(&cn_doc)
    );

    // 1. The new voucher draft: supplier contact, a position comment.
    may_create();
    let mut draft = ctx.draft();
    draft.supplier_name = String::new();
    draft.supplier = Some(ObjectRef::new(customer.contact.id));
    let saved = ctx
        .c
        .save_voucher(&SaveVoucher {
            voucher: draft.clone(),
            positions: vec![position(35, COMMENT)],
            filename: None,
        })
        .await
        .unwrap();
    let voucher_id = saved.voucher.id.get();
    created("Voucher", voucher_id);
    draft.id = Some(saved.voucher.id);
    let mut voucher = VoucherSetup {
        id: voucher_id,
        base: serde_json::to_value(SaveVoucher {
            voucher: draft,
            positions: with_ids(&saved, &[(35, COMMENT)]),
            filename: None,
        })
        .unwrap(),
    };
    notes.insert(
        "voucher",
        "new draft with supplier contact and position comment".into(),
    );

    // 2. One tag, created on the voucher; then attached to the others.
    may_create();
    let tag_name = format!("{}-tag", ctx.marker);
    let (status, answer) = write(
        &ctx,
        Method::POST,
        "/Tag/Factory/create",
        json!({"name": tag_name, "object": object_ref(voucher_id, "Voucher")}),
    )
    .await;
    // An error answer does not mean "not written": look the tag up either way.
    let tags = list(&ctx, "/Tag", &pairs(&[("name", &tag_name)])).await;
    let tag_id = tags
        .unwrap_or_default()
        .iter()
        .find(|t| text(t, "name").as_deref() == Some(tag_name.as_str()))
        .and_then(|t| sid(&t["id"]));
    let tag_id =
        tag_id.unwrap_or_else(|| panic!("no tag after Tag/Factory/create (answer {status})"));
    created("Tag", &tag_id);
    notes.insert(
        "tag Voucher",
        format!("created with the tag ({})", error_or_ok(status, &answer)),
    );
    for (kind, id) in [
        ("Invoice", inv_id),
        ("CreditNote", cn_id),
        ("Contact", Some(contact_id)),
    ] {
        let key = tag_key(kind);
        let Some(id) = id else {
            notes.insert(key, "no probe object found".into());
            continue;
        };
        may_create();
        let (status, answer) = write(
            &ctx,
            Method::POST,
            "/TagRelation",
            json!({
                "tag": {"id": tag_id.parse::<i64>().unwrap(), "objectName": "Tag"},
                "object": object_ref(id, kind),
            }),
        )
        .await;
        if failed(status, &answer) {
            notes.insert(
                key,
                format!("POST /TagRelation refused: {}", error_note(status, &answer)),
            );
        } else {
            let rel = first_object(&answer)
                .and_then(|o| sid(&o["id"]))
                .unwrap_or_else(|| "?".into());
            created("TagRelation", rel);
            notes.insert(key, "attached with POST /TagRelation".into());
        }
    }

    // 3. One cost centre (no route in the spec), set where drafts allow it.
    let cc_name = format!("{}-cc", ctx.marker);
    let existing = list(&ctx, "/CostCentre", &none).await;
    let cc_id: Option<String> = match existing {
        Ok(l) if !l.is_empty() => {
            notes.insert("cc", "existing cost centre".into());
            sid(&l[0]["id"])
        }
        other => {
            let read = other.err().unwrap_or_else(|| "empty".into());
            may_create();
            let (status, answer) =
                write(&ctx, Method::POST, "/CostCentre", json!({"name": cc_name})).await;
            if failed(status, &answer) {
                notes.insert(
                    "cc",
                    format!(
                        "GET: {read}; POST /CostCentre: {}",
                        error_note(status, &answer)
                    ),
                );
                None
            } else {
                let id = first_object(&answer).and_then(|o| sid(&o["id"]));
                if let Some(id) = &id {
                    created("CostCentre", id);
                }
                notes.insert("cc", "created with POST /CostCentre".into());
                id
            }
        }
    };
    if let Some(cc) = cc_id.as_ref().and_then(|s| s.parse::<i64>().ok()) {
        let reference = object_ref(cc, "CostCentre");
        let inv_note =
            put_cost_centre(&ctx, "Invoice", inv_id, is_draft(&inv_doc), &reference).await;
        let cn_note =
            put_cost_centre(&ctx, "CreditNote", cn_id, is_draft(&cn_doc), &reference).await;
        notes.insert("cc Invoice", inv_note);
        notes.insert("cc CreditNote", cn_note);
        let v_note = voucher
            .update(&ctx, &[("costCentre", reference.clone())])
            .await;
        notes.insert("cc Voucher", format!("save factory: {v_note}"));
    }

    // 4. A past payment deadline on the voucher draft (dd.mm.yyyy, as measured).
    let deadline = voucher
        .update(&ctx, &[("paymentDeadline", json!("01.06.2026"))])
        .await;
    notes.insert(
        "deadline",
        format!("paymentDeadline 01.06.2026: {deadline}"),
    );

    // 5. A recurring invoice draft: one write, no number expected.
    let mut wkr_id: Option<String> = None;
    may_create();
    let save = SaveInvoice::new(
        invoice_draft(&InvoiceSpec {
            customer: &customer,
            person,
            note: format!("{}-wkr", ctx.marker),
            b2b: false,
            payment_method: None,
            with_address: false,
        }),
        vec![net_position(1, 1000, "SDRS-FILTER2 Position", None)],
    );
    let mut body = serde_json::to_value(&save).unwrap();
    body["invoice"]["invoiceType"] = "WKR".into();
    let (status, answer) = write(&ctx, Method::POST, "/Invoice/Factory/saveInvoice", body).await;
    if failed(status, &answer) {
        notes.insert(
            "wkr",
            format!("WKR draft refused: {}", error_note(status, &answer)),
        );
    } else {
        wkr_id = serde_json::from_str::<Value>(&answer)
            .ok()
            .and_then(|v| sid(&v["objects"]["invoice"]["id"]));
        if let Some(id) = &wkr_id {
            created("Invoice(WKR)", id);
            let (s, t) = ctx
                .raw(Method::GET, &format!("/Invoice/{id}"), None, None)
                .await;
            let number = first_object(&t).and_then(|o| text(&o, "invoiceNumber"));
            eprintln!(
                "NUMBER-CHECK Invoice {id} http {s} invoiceNumber present: {}",
                number.as_deref().is_some_and(|n| !n.is_empty())
            );
            notes.insert("wkr", "WKR draft created with one write".into());
        }
    }
    eprintln!(
        "OBJECTS created {} (budget {BUDGET_OBJECTS})",
        CREATED.load(Ordering::SeqCst)
    );

    // Measurements.
    let mut results = Results::default();
    let invoices = list(&ctx, "/Invoice", &none).await.unwrap();
    let credit_notes = list(&ctx, "/CreditNote", &none).await.unwrap();
    let mut vouchers = list(&ctx, "/Voucher", &none).await.unwrap();
    let contacts = list(&ctx, "/Contact", &none).await.unwrap();
    let relations = list(&ctx, "/TagRelation", &none).await.unwrap_or_default();

    // Tags.
    for (endpoint, unf) in [
        ("Invoice", &invoices),
        ("CreditNote", &credit_notes),
        ("Voucher", &vouchers),
        ("Contact", &contacts),
    ] {
        let tagged: std::collections::BTreeSet<String> = relations
            .iter()
            .filter(|r| {
                ref_id(r, "tag").as_deref() == Some(tag_id.as_str())
                    && r["object"]["objectName"].as_str() == Some(endpoint)
            })
            .filter_map(|r| ref_id(r, "object"))
            .collect();
        let hyps = [hyp("carries the tag", unf, |o| {
            sid(&o["id"]).is_some_and(|i| tagged.contains(&i))
        })];
        let setup = notes.get(tag_key(endpoint)).cloned().unwrap_or_default();
        for form in ["tags[0]", "tags[x]", "tags"] {
            let query = pairs(&[
                (&format!("{form}[id]"), &tag_id),
                (&format!("{form}[objectName]"), "Tag"),
            ]);
            results
                .measure(
                    &ctx,
                    Probe {
                        endpoint,
                        filter: "tags[x][id]",
                        setup: &setup,
                        form: &format!("{form}[id]&{form}[objectName]=Tag"),
                        query,
                    },
                    unf,
                    &hyps,
                )
                .await;
        }
    }

    // Cost centre.
    if let Some(cc) = &cc_id {
        for (endpoint, unf) in [
            ("Invoice", &invoices),
            ("CreditNote", &credit_notes),
            ("Voucher", &vouchers),
        ] {
            let hyps = [hyp("costCentre.id equals", unf, |o| {
                ref_id(o, "costCentre").as_deref() == Some(cc.as_str())
            })];
            let setup = notes.get(cc_key(endpoint)).cloned().unwrap_or_default();
            for (form, query) in [
                (
                    "costCentre[id]&costCentre[objectName]=CostCentre",
                    pairs(&[
                        ("costCentre[id]", cc),
                        ("costCentre[objectName]", "CostCentre"),
                    ]),
                ),
                ("costCentre[id]", pairs(&[("costCentre[id]", cc)])),
            ] {
                results
                    .measure(
                        &ctx,
                        Probe {
                            endpoint,
                            filter: "costCentre[id]",
                            setup: &setup,
                            form,
                            query,
                        },
                        unf,
                        &hyps,
                    )
                    .await;
            }
        }
    } else {
        for endpoint in ["Invoice", "CreditNote", "Voucher"] {
            results.skipped(
                endpoint,
                "costCentre[id]",
                "untestable",
                notes.get("cc").map_or("no cost centre", String::as_str),
            );
        }
    }

    // Voucher supplier.
    let cid = contact_id.to_string();
    let hyps = [hyp("supplier.id equals", &vouchers, |o| {
        ref_id(o, "supplier").as_deref() == Some(cid.as_str())
    })];
    for (filter, key) in [
        ("contact[id]", "contact"),
        ("contactOrObject[id]", "contactOrObject"),
        ("supplier[id]", "supplier"),
    ] {
        results
            .measure(
                &ctx,
                Probe {
                    endpoint: "Voucher",
                    filter,
                    setup: "new voucher draft with a supplier contact",
                    form: &format!("{key}[id]&{key}[objectName]=Contact"),
                    query: pairs(&[
                        (&format!("{key}[id]"), &cid),
                        (&format!("{key}[objectName]"), "Contact"),
                    ]),
                },
                &vouchers,
                &hyps,
            )
            .await;
    }

    // Voucher commentLike, against the positions' comments.
    let positions = list(&ctx, "/VoucherPos", &none).await.unwrap_or_default();
    let having = |matches: &dyn Fn(&str) -> bool| -> std::collections::BTreeSet<String> {
        positions
            .iter()
            .filter(|p| text(p, "comment").is_some_and(|c| matches(&c)))
            .filter_map(|p| ref_id(p, "voucher"))
            .collect()
    };
    let contains = having(&|c| c.contains(COMMENT_PART));
    let contains_ci = having(&|c| c.to_lowercase().contains(&COMMENT_PART.to_lowercase()));
    let hyps = [
        hyp("a position comment contains the value", &vouchers, |o| {
            sid(&o["id"]).is_some_and(|i| contains.contains(&i))
        }),
        hyp("... case-insensitive", &vouchers, |o| {
            sid(&o["id"]).is_some_and(|i| contains_ci.contains(&i))
        }),
    ];
    for (form, value) in [
        ("part of the comment", COMMENT_PART.to_owned()),
        ("whole comment", COMMENT.to_owned()),
        ("part, lower case", COMMENT_PART.to_lowercase()),
    ] {
        results
            .measure(
                &ctx,
                Probe {
                    endpoint: "Voucher",
                    filter: "commentLike",
                    setup: "new voucher draft with a position comment",
                    form,
                    query: pairs(&[("commentLike", &value)]),
                },
                &vouchers,
                &hyps,
            )
            .await;
    }

    // Voucher delinquent: payment deadline in the past.
    let now = today();
    let past = |o: &Value| {
        text(o, "paymentDeadline").is_some_and(|d| d.chars().take(10).collect::<String>() < now)
    };
    let status_is = |o: &Value, s: &str| text(o, "status").as_deref() == Some(s);
    let hyps = [
        hyp("paymentDeadline before today", &vouchers, past),
        hyp("deadline before today and status 100", &vouchers, |o| {
            past(o) && status_is(o, "100")
        }),
        hyp("deadline before today and status 50", &vouchers, |o| {
            past(o) && status_is(o, "50")
        }),
    ];
    results
        .measure(
            &ctx,
            Probe {
                endpoint: "Voucher",
                filter: "delinquent",
                setup: &notes["deadline"],
                form: "delinquent=true",
                query: pairs(&[("delinquent", "true")]),
            },
            &vouchers,
            &hyps,
        )
        .await;

    // Recurring invoices.
    let wkr = wkr_id.clone();
    let setup = notes.get("wkr").cloned().unwrap_or_default();
    for value in ["all", "true", "1"] {
        let got = results
            .measure(
                &ctx,
                Probe {
                    endpoint: "Invoice",
                    filter: "showWkr",
                    setup: &setup,
                    form: &format!("showWkr={value}"),
                    query: pairs(&[("showWkr", value)]),
                },
                &invoices,
                &[],
            )
            .await;
        if let (Some(got), Some(id)) = (got, &wkr) {
            let default = invoices
                .iter()
                .any(|o| sid(&o["id"]).as_deref() == Some(id));
            let shown = got.iter().any(|o| sid(&o["id"]).as_deref() == Some(id));
            eprintln!(
                "FILTER2\tInvoice\tshowWkr\tshowWkr={value}\tWKR draft in default list: {default}, in filtered list: {shown}, extra objects: {}",
                got.len().saturating_sub(invoices.len())
            );
        }
    }
    results.skipped(
        "Invoice",
        "showMa",
        "not-done",
        "needs a finalized document (a dunning run)",
    );
    results.skipped(
        "Invoice",
        "onlyDunned",
        "not-done",
        "needs a finalized document (a dunning run)",
    );
    results.skipped(
        "CreditNote",
        "partiallyPaid",
        "not-done",
        "needs a finalized document with a booked part payment",
    );

    // Day: a known invoice date, in several forms.
    for (endpoint, field, unf, doc) in [
        ("Invoice", "invoiceDate", &invoices, &inv_doc),
        ("CreditNote", "creditNoteDate", &credit_notes, &cn_doc),
    ] {
        day_probe(&ctx, &mut results, endpoint, field, unf, doc).await;
    }

    // Voucher type RV last: noRv / withoutCatering.
    let rv = voucher.update(&ctx, &[("voucherType", json!("RV"))]).await;
    notes.insert("rv", format!("voucherType RV on the new draft: {rv}"));
    vouchers = list(&ctx, "/Voucher", &none).await.unwrap();
    let not_rv = |o: &Value| text(o, "voucherType").as_deref() != Some("RV");
    let hyps = [hyp("voucherType is not RV", &vouchers, not_rv)];
    for filter in ["noRv", "withoutCatering"] {
        results
            .measure(
                &ctx,
                Probe {
                    endpoint: "Voucher",
                    filter,
                    setup: &notes["rv"],
                    form: &format!("{filter}=true"),
                    query: pairs(&[(filter, "true")]),
                },
                &vouchers,
                &hyps,
            )
            .await;
    }
    results.write();
    eprintln!(
        "OBJECTS created {} (budget {BUDGET_OBJECTS})",
        CREATED.load(Ordering::SeqCst)
    );
    eprintln!("FILTER2\tdone");
}

async fn put_cost_centre(
    ctx: &Ctx,
    kind: &str,
    id: Option<i64>,
    draft: bool,
    reference: &Value,
) -> String {
    let Some(id) = id.filter(|_| draft) else {
        return "no probe draft found".to_owned();
    };
    let (status, answer) = write(
        ctx,
        Method::PUT,
        &format!("/{kind}/{id}"),
        json!({"costCentre": reference}),
    )
    .await;
    if failed(status, &answer) {
        format!("PUT refused: {}", error_note(status, &answer))
    } else {
        "PUT answered ok".to_owned()
    }
}

fn error_or_ok(status: u16, answer: &str) -> String {
    if failed(status, answer) {
        format!("answer was an error: {}", error_note(status, answer))
    } else {
        "answer ok".into()
    }
}

fn tag_key(kind: &str) -> &'static str {
    match kind {
        "Invoice" => "tag Invoice",
        "CreditNote" => "tag CreditNote",
        "Contact" => "tag Contact",
        _ => "tag Voucher",
    }
}

fn cc_key(kind: &str) -> &'static str {
    match kind {
        "Invoice" => "cc Invoice",
        "CreditNote" => "cc CreditNote",
        _ => "cc Voucher",
    }
}

/// `day={timestamp}` with a known date: the stored instant, noon, UTC midnight
/// and two date texts. Judged against "same local day as the field".
async fn day_probe(
    ctx: &Ctx,
    results: &mut Results,
    endpoint: &str,
    field: &str,
    unf: &[Value],
    doc: &Option<Value>,
) {
    let known = doc
        .as_ref()
        .and_then(|d| text(d, field))
        .or_else(|| unf.iter().find_map(|o| text(o, field)));
    let Some(stored) = known else {
        results.skipped(endpoint, "day", "untestable", "no object with a date");
        return;
    };
    let Ok(at) = OffsetDateTime::parse(&stored, &Rfc3339) else {
        results.skipped(endpoint, "day", "untestable", "the date is not RFC 3339");
        return;
    };
    let day: String = stored.chars().take(10).collect();
    let hyps = [hyp("same local day", unf, |o| {
        text(o, field).is_some_and(|s| s.chars().take(10).collect::<String>() == day)
    })];
    let utc_midnight = at.date().midnight().assume_utc();
    let forms: [(&str, String); 5] = [
        (
            "stored instant (Berlin midnight)",
            at.unix_timestamp().to_string(),
        ),
        (
            "stored instant + 12h (noon)",
            (at + Duration::hours(12)).unix_timestamp().to_string(),
        ),
        (
            "UTC midnight of the date",
            utc_midnight.unix_timestamp().to_string(),
        ),
        ("date text yyyy-mm-dd", day.clone()),
        (
            "date text dd.mm.yyyy",
            format!("{}.{}.{}", &day[8..10], &day[5..7], &day[0..4]),
        ),
    ];
    for (form, value) in forms {
        results
            .measure(
                ctx,
                Probe {
                    endpoint,
                    filter: "day",
                    setup: "known date of a probe document",
                    form,
                    query: pairs(&[("day", &value)]),
                },
                unf,
                &hyps,
            )
            .await;
    }
    if !results.classes(endpoint, "day").contains(&"exact") {
        results.skipped(
            endpoint,
            "day",
            "summary",
            "spec form does not work: no value form fits",
        );
    }
}
