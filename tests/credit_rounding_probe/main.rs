//! The credit rounding probe: measures where sevDesk rounds the tax of a net
//! document with two positions of EUR 0.03 net at 19 % (0.0057 per position:
//! 0.01 per line, 0.01 on the sum of 0.06 net is 0.0114 -> 0.01, per line
//! rounded twice it is 0.02), and whether a full credit note of that invoice
//! comes out with the same gross, as a draft and as a finalized document.
//! Writes 2 PERMANENT documents to the sevDesk TEST account (the probe aborts
//! on any other account before the first write), so it is its own test target
//! (`test = false` in Cargo.toml) and runs only on request:
//!
//! `just test-live --test credit_rounding_probe case1`
//!
//! `case2_half_cent_drafts` writes and deletes two credit note DRAFTS (net
//! 1.50 and 0.50, VAT exactly 0.285 and 0.095) to see how a half cent rounds;
//! it leaves nothing behind and consumes no number.
//!
//! Sequence (all writes through the crate's public API, every read raw):
//! 1. `GET /SevClient` (account check), then read-only inputs: contact,
//!    contact person, payment method.
//! 2. `save_invoice` (net, two positions) -> `send_invoice_by(FINALIZE)`
//!    (consumes ONE RE number) -> raw `GET /Invoice/{id}` and
//!    `GET /InvoicePos?invoice[id]=..&invoice[objectName]=Invoice`.
//! 3. `next_credit_note_number` (read-only) -> `save_credit_note` (full credit
//!    of that invoice, same positions) -> raw read of head and positions.
//! 4. `send_credit_note_by(FINALIZE)` (consumes ONE GU number) -> raw read.
//!
//! Budget: 2 permanent documents, at most 1 RE and 1 GU number. No write is
//! retried; after a failed write the answer may still have been written, so
//! the run re-reads by the run marker and stops.
//! Output: scrubbed raw answers in `target/tmp/captures/credit_rounding_*.json`
//! and the stderr lines `CREATED`, `NUMBER`, `ROUND` and `ROUND-VERDICT`.
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
#[path = "../common/scrub.rs"]
mod scrub;

use docs::*;
use harness::*;
use reqwest::Method;
use serde_json::Value;
use sevdesk::*;
use time::OffsetDateTime;
use time::macros::{date, datetime};

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;
/// The name prefix of the contacts the live tests create.
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";
/// Net price of each of the two positions, in cents.
const NET: i64 = 3;

async fn verify_test_account(ctx: &Ctx) {
    let (status, text) = ctx.raw(Method::GET, "/SevClient", None, None).await;
    assert!(status < 300, "GET /SevClient answered {status}");
    let id = raw_one(&text)["id"]
        .as_str()
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
        .unwrap_or_else(|| panic!("no existing contact named `{CONTACT_PREFIX}…`: aborting"));
    let verified = ctx.c.contact(contact.id).await.unwrap();
    assert_eq!(verified.id, contact.id);
    let name = verified.name.clone().expect("the contact has a name");
    Customer {
        contact: verified,
        name,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

fn text_of(v: &Value, key: &str) -> String {
    match &v[key] {
        Value::Null => "-".to_owned(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The two net positions: quantity 1, EUR 0.03 net, 19 %.
fn positions() -> Vec<DocumentPositionDraft> {
    vec![
        net_position(1, NET, "SDRS-RND Position A", None),
        net_position(1, NET, "SDRS-RND Position B", None),
    ]
}

/// The sums of one document read back raw.
struct Sums {
    id: String,
    number: String,
    status: String,
    enshrined: String,
    net: String,
    tax: String,
    gross: String,
    /// `net/tax/gross` per position.
    positions: Vec<[String; 3]>,
}

impl Sums {
    fn positions_text(&self) -> String {
        self.positions
            .iter()
            .map(|p| p.join("/"))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// Reads head and positions of a document raw and records both under
/// `credit_rounding_<name>_head` / `_pos`. `None` when a read failed.
async fn read_sums(ctx: &Ctx, obj: &str, id: i64, name: &str) -> Option<Sums> {
    read_sums_at(ctx, obj, id, &format!("credit_rounding_{name}")).await
}

/// [`read_sums`] with the full capture name prefix (`<prefix>_head` / `_pos`).
async fn read_sums_at(ctx: &Ctx, obj: &str, id: i64, prefix: &str) -> Option<Sums> {
    let number_key = if obj == "Invoice" {
        "invoiceNumber"
    } else {
        "creditNoteNumber"
    };
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}/{id}"),
            None,
            Some(&format!("{prefix}_head")),
        )
        .await;
    if status >= 300 {
        eprintln!("ROUND-FAILED\tGET /{obj}/{id} answered {status}");
        return None;
    }
    let head = raw_one(&text);
    let ref_key = if obj == "Invoice" {
        "invoice"
    } else {
        "creditNote"
    };
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}Pos?{ref_key}[id]={id}&{ref_key}[objectName]={obj}"),
            None,
            Some(&format!("{prefix}_pos")),
        )
        .await;
    if status >= 300 {
        eprintln!("ROUND-FAILED\tGET /{obj}Pos answered {status}");
        return None;
    }
    let list: Vec<Value> = serde_json::from_value(raw_objects(&text)).unwrap_or_default();
    Some(Sums {
        id: id.to_string(),
        number: text_of(&head, number_key),
        status: text_of(&head, "status"),
        enshrined: text_of(&head, "enshrined"),
        net: text_of(&head, "sumNet"),
        tax: text_of(&head, "sumTax"),
        gross: text_of(&head, "sumGross"),
        positions: list
            .iter()
            .map(|p| {
                [
                    text_of(p, "sumNet"),
                    text_of(p, "sumTax"),
                    text_of(p, "sumGross"),
                ]
            })
            .collect(),
    })
}

fn print_round(doc: &str, state: &str, s: &Sums) {
    eprintln!(
        "ROUND\t{doc}\t{state}\t{}\t{}\t{}\t{}\t{}\t{}",
        s.id,
        s.number,
        s.net,
        s.tax,
        s.gross,
        s.positions_text()
    );
    eprintln!(
        "NUMBER {doc} {} {}\tstatus {}\tenshrined {}",
        s.id, s.number, s.status, s.enshrined
    );
}

/// Where the tax of a document was rounded: per line (every position carries
/// 0.02 in total) or on the sum.
fn tax_where(s: &Sums) -> String {
    let per_line_sum: Option<f64> = s.positions.iter().map(|p| p[1].parse::<f64>().ok()).sum();
    format!(
        "head tax {} / positions' tax {:?} (sum {:?}): 0.02 = per line, 0.01 = on sum",
        s.tax,
        s.positions
            .iter()
            .map(|p| p[1].as_str())
            .collect::<Vec<_>>(),
        per_line_sum
    )
}

/// After a failed write: finds what may have been written by the run marker
/// and stops. Nothing is retried.
async fn reread_and_stop(ctx: &Ctx, obj: &str, what: &str, err: &dyn std::fmt::Display) -> ! {
    eprintln!("ROUND-FAILED\t{what} failed: {err}");
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/{obj}?customerInternalNote={}", ctx.marker),
            None,
            Some(&format!("credit_rounding_{what}_reread")),
        )
        .await;
    let head: String = scrub::scrub_capture(&text).chars().take(400).collect();
    eprintln!(
        "ROUND-FAILED\tre-read HTTP {status}\t{}",
        head.replace('\n', " ")
    );
    panic!(
        "{what} failed, stopped without retry; marker {}",
        ctx.marker
    );
}

#[tokio::test]
#[ignore = "writes PERMANENT documents to the sevDesk test account"]
async fn case1_credit_rounding() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the credit rounding probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-RND-{ts}"));
    verify_test_account(&ctx).await;

    // Read-only inputs.
    let methods = ctx.c.list_payment_methods().await.unwrap();
    let pm = methods
        .iter()
        .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
        .expect("a payment method with electronicInvoiceId 68")
        .id;
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    eprintln!(
        "SETUP contact {} payment method {}",
        customer.contact.id.get(),
        pm.get()
    );

    // 1. The net invoice: draft, finalize, read back.
    let note = format!("{}-invoice", ctx.marker);
    let spec = InvoiceSpec {
        customer: &customer,
        person,
        note,
        b2b: true,
        payment_method: Some(pm),
        with_address: true,
    };
    let saved = match ctx
        .c
        .save_invoice(&SaveInvoice::new(invoice_draft(&spec), positions()))
        .await
    {
        Ok(s) => s,
        Err(e) => reread_and_stop(&ctx, "Invoice", "save_invoice", &e).await,
    };
    let invoice_id = saved.invoice.id;
    eprintln!("CREATED Invoice {}", invoice_id.get());
    if let Err(e) = ctx.c.send_invoice_by(invoice_id, &SendBy::FINALIZE).await {
        // The finalize may have been written: show the document, then stop.
        let _ = read_sums(
            &ctx,
            "Invoice",
            invoice_id.get(),
            "invoice_after_failed_send",
        )
        .await;
        panic!("send_invoice_by failed ({e}), stopped without retry");
    }
    let invoice = read_sums(&ctx, "Invoice", invoice_id.get(), "invoice_final")
        .await
        .expect("the finalized invoice reads back");
    print_round("Invoice", "final", &invoice);

    // 2. The full credit note draft of that invoice, read back.
    let number = ctx.c.next_credit_note_number().await.unwrap();
    let note = format!("{}-credit-note", ctx.marker);
    let draft = {
        let mut draft = CreditNoteDraft {
            credit_note_number: number,
            contact: ObjectRef::new(customer.contact.id),
            contact_person: ObjectRef::new(person),
            credit_note_date: date!(2026 - 10 - 01),
            delivery_date: datetime!(2026-10-01 0:00 +2),
            header: note.clone(),
            tax_rule: ObjectRef::new(TaxRuleId::new(1)),
            tax_text: "Umsatzsteuer 19%".into(),
            booking_category: CreditNoteCategory::Underachievement,
            currency: CurrencyCode::EUR,
            show_net: true,
            customer_internal_note: note,
            options: Default::default(),
        };
        draft.options.delivery_date_until = Some(datetime!(2026-10-31 0:00 +1));
        draft.options.ref_src_invoice = Some(ObjectRef::new(invoice_id));
        draft.options.address_name = Some(customer.name.clone());
        draft.options.address_street = Some(customer.street.into());
        draft.options.address_zip = Some(customer.zip.into());
        draft.options.address_city = Some(customer.city.into());
        draft.options.address_country = Some(ObjectRef::new(GERMANY));
        draft.options.payment_method = Some(ObjectRef::new(pm));
        draft.options.is_e_invoice = true;
        draft
    };
    let saved = match ctx
        .c
        .save_credit_note(&SaveCreditNote::new(draft, positions()))
        .await
    {
        Ok(s) => s,
        Err(e) => reread_and_stop(&ctx, "CreditNote", "save_credit_note", &e).await,
    };
    let credit_id = saved.credit_note.id;
    eprintln!("CREATED CreditNote {}", credit_id.get());
    let credit_draft = read_sums(&ctx, "CreditNote", credit_id.get(), "credit_draft")
        .await
        .expect("the credit note draft reads back");
    print_round("CreditNote", "draft", &credit_draft);

    // 3. Finalize the credit note and read it back.
    if let Err(e) = ctx
        .c
        .send_credit_note_by(credit_id, &SendBy::FINALIZE)
        .await
    {
        let _ = read_sums(
            &ctx,
            "CreditNote",
            credit_id.get(),
            "credit_after_failed_send",
        )
        .await;
        panic!("send_credit_note_by failed ({e}), stopped without retry");
    }
    let credit_final = read_sums(&ctx, "CreditNote", credit_id.get(), "credit_final")
        .await
        .expect("the finalized credit note reads back");
    print_round("CreditNote", "final", &credit_final);

    eprintln!(
        "ROUND-VERDICT\tcredit gross {} vs invoice gross {}: equal={}\t\
         invoice tax: {}\tcredit draft tax: {}\tcredit final tax: {}\t\
         credit draft vs final sums equal={} (net {}/{}, tax {}/{}, gross {}/{}, positions {}/{})",
        credit_final.gross,
        invoice.gross,
        credit_final.gross == invoice.gross,
        tax_where(&invoice),
        tax_where(&credit_draft),
        tax_where(&credit_final),
        (
            &credit_draft.net,
            &credit_draft.tax,
            &credit_draft.gross,
            &credit_draft.positions
        ) == (
            &credit_final.net,
            &credit_final.tax,
            &credit_final.gross,
            &credit_final.positions
        ),
        credit_draft.net,
        credit_final.net,
        credit_draft.tax,
        credit_final.tax,
        credit_draft.gross,
        credit_final.gross,
        credit_draft.positions_text(),
        credit_final.positions_text(),
    );
}

/// Gross needed on the reference invoice for `case2`, in thousandths: both
/// drafts together (1.79 + 0.60 = 2.39); the drafts count against the cap too.
/// `case3` passes its own figure.
const NEEDED_GROSS_THOUSANDTHS: i64 = 2390;

/// Reference invoice for `case2`: an enshrined net invoice whose gross minus
/// the gross of all credit notes already referencing it covers
/// [`NEEDED_GROSS_THOUSANDTHS`]; one without any credit note is preferred.
/// Read-only; aborts when none qualifies.
async fn enshrined_invoice(ctx: &Ctx, needed_thousandths: i64) -> InvoiceId {
    let enshrined = |v: &Value| !matches!(v["enshrined"], Value::Null);
    let (status, text) = ctx
        .raw(Method::GET, "/Invoice?limit=1000", None, None)
        .await;
    assert!(status < 300, "GET /Invoice answered {status}");
    let invoices: Vec<Value> = serde_json::from_value(raw_objects(&text)).unwrap();
    let (status, text) = ctx
        .raw(Method::GET, "/CreditNote?limit=1000", None, None)
        .await;
    assert!(status < 300, "GET /CreditNote answered {status}");
    let credits: Vec<Value> = serde_json::from_value(raw_objects(&text)).unwrap_or_default();
    // (invoice id, credit notes referencing it, room left in thousandths)
    let mut best: Option<(i64, usize, i64)> = None;
    // Skip counts per reason, printed when nothing qualifies.
    let (mut not_enshrined, mut not_net, mut not_re, mut bad_id, mut bad_gross) = (0, 0, 0, 0, 0);
    let (mut bad_credit, mut no_room) = (0, 0);
    for inv in &invoices {
        if !enshrined(inv) {
            not_enshrined += 1;
            continue;
        }
        // sevDesk answers the flag as "1" (measured); "true" covers a JSON bool.
        if !matches!(text_of(inv, "showNet").as_str(), "1" | "true") {
            not_net += 1;
            continue;
        }
        if text_of(inv, "invoiceType") != "RE" {
            not_re += 1;
            continue;
        }
        let Some(id) = inv["id"].as_str().and_then(|s| s.parse::<i64>().ok()) else {
            bad_id += 1;
            continue;
        };
        let Some(gross) = thousandths(&text_of(inv, "sumGross")) else {
            bad_gross += 1;
            continue;
        };
        let refs: Vec<&Value> = credits
            .iter()
            .filter(|c| c["refSrcInvoice"]["id"].as_str() == Some(&id.to_string()))
            .collect();
        let used: Option<i64> = refs
            .iter()
            .map(|c| thousandths(&text_of(c, "sumGross")))
            .sum();
        let Some(used) = used else {
            bad_credit += 1;
            continue;
        };
        let room = gross - used;
        if room < needed_thousandths {
            no_room += 1;
            continue;
        }
        let better = best.is_none_or(|(_, n, r)| (refs.len(), -room) < (n, -r));
        if better {
            best = Some((id, refs.len(), room));
        }
    }
    let Some((id, n, room)) = best else {
        eprintln!(
            "REF-INVOICE\tnone of {} invoices qualifies (and {} credit notes read): \
             not enshrined {not_enshrined}, not net {not_net}, not RE {not_re}, \
             unreadable id {bad_id}, unreadable gross {bad_gross}, \
             unreadable credit note gross {bad_credit}, too little room {no_room}",
            invoices.len(),
            credits.len(),
        );
        panic!("no enshrined net invoice with enough room below its gross: aborting");
    };
    eprintln!("REF-INVOICE\t{id}: {n} credit notes, room {room} thousandths");
    InvoiceId::new(id)
}

/// The raw answer of the CN sequence (scrubbed for comparison) and the number
/// the crate derives from it.
async fn credit_sequence(ctx: &Ctx, name: &str) -> (String, String) {
    let (status, text) = ctx
        .raw(
            Method::GET,
            "/SevSequence/Factory/getByType?objectType=CreditNote&type=CN",
            None,
            Some(name),
        )
        .await;
    assert!(status < 300, "GET /SevSequence answered {status}");
    let next = ctx.c.next_credit_note_number().await.unwrap();
    (scrub::scrub_capture(&text), next)
}

/// Thousandths of a decimal string, parsed exactly (`"0.285"` -> 285,
/// `"-5.95"` -> -5950); `None` for anything but up to three decimals.
fn thousandths(s: &str) -> Option<i64> {
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    if int.is_empty() || frac.len() > 3 || !all_digits(int) || !all_digits(frac) {
        return None;
    }
    let frac_thousandths: i64 = format!("{frac:0<3}").parse().ok()?;
    let v = int.parse::<i64>().ok()?.checked_mul(1000)? + frac_thousandths;
    Some(if neg { -v } else { v })
}

#[test]
fn thousandths_parses_sevdesk_amounts_exactly() {
    for (text, want) in [
        ("65.45", Some(65_450)),
        ("11.9", Some(11_900)),
        ("238", Some(238_000)),
        ("-5.95", Some(-5_950)),
        ("0.285", Some(285)),
        ("2.39", Some(2_390)),
        ("-", None),
        ("", None),
        ("1.2345", None),
        ("1e3", None),
    ] {
        assert_eq!(thousandths(text), want, "{text}");
    }
    // A JSON number reaches the parser through `text_of`.
    let v: Value = serde_json::from_str(r#"{"a": 11.9, "b": 238}"#).unwrap();
    assert_eq!(thousandths(&text_of(&v, "a")), Some(11_900));
    assert_eq!(thousandths(&text_of(&v, "b")), Some(238_000));
}

/// Raw list of credit notes carrying `marker`, filtered client-side because
/// the filter on `customerInternalNote` is not trusted.
async fn drafts_by_marker(ctx: &Ctx, marker: &str, name: &str) -> Vec<String> {
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/CreditNote?customerInternalNote={marker}&limit=1000"),
            None,
            Some(name),
        )
        .await;
    assert!(status < 300, "GET /CreditNote by marker answered {status}");
    let list: Vec<Value> = serde_json::from_value(raw_objects(&text)).unwrap_or_default();
    list.iter()
        .filter(|c| text_of(c, "customerInternalNote").contains(marker))
        .map(|c| text_of(c, "id"))
        .collect()
}

#[tokio::test]
#[ignore = "writes and deletes credit note drafts on the sevDesk test account"]
async fn case2_half_cent_drafts() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the half cent probe");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-HALF-{ts}"));
    verify_test_account(&ctx).await;

    // Read-only inputs.
    let methods = ctx.c.list_payment_methods().await.unwrap();
    let pm = methods
        .iter()
        .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
        .expect("a payment method with electronicInvoiceId 68")
        .id;
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    let invoice_id = enshrined_invoice(&ctx, NEEDED_GROSS_THOUSANDTHS).await;
    let (seq_before, number_before) = credit_sequence(&ctx, "half_cent_sequence_before").await;
    eprintln!(
        "SETUP contact {} payment method {} ref invoice {} next CN {number_before}",
        customer.contact.id.get(),
        pm.get(),
        invoice_id.get()
    );

    // (case, net price in cents, expected half-up tax, expected half-even tax)
    let cases = [("a", 150_i64, "0.29", "0.28"), ("b", 50, "0.10", "0.10")];
    let mut created: Vec<(&str, i64, String)> = Vec::new();
    for (case, net, _, _) in cases {
        let note = format!("{}-{case}", ctx.marker);
        let mut draft = CreditNoteDraft {
            credit_note_number: number_before.clone(),
            contact: ObjectRef::new(customer.contact.id),
            contact_person: ObjectRef::new(person),
            credit_note_date: date!(2026 - 10 - 01),
            delivery_date: datetime!(2026-10-01 0:00 +2),
            header: note.clone(),
            tax_rule: ObjectRef::new(TaxRuleId::new(1)),
            tax_text: "Umsatzsteuer 19%".into(),
            booking_category: CreditNoteCategory::Underachievement,
            currency: CurrencyCode::EUR,
            show_net: true,
            customer_internal_note: note.clone(),
            options: Default::default(),
        };
        draft.options.delivery_date_until = Some(datetime!(2026-10-31 0:00 +1));
        draft.options.ref_src_invoice = Some(ObjectRef::new(invoice_id));
        draft.options.address_name = Some(customer.name.clone());
        draft.options.address_street = Some(customer.street.into());
        draft.options.address_zip = Some(customer.zip.into());
        draft.options.address_city = Some(customer.city.into());
        draft.options.address_country = Some(ObjectRef::new(GERMANY));
        draft.options.payment_method = Some(ObjectRef::new(pm));
        draft.options.is_e_invoice = true;
        let positions = vec![net_position(
            1,
            net,
            &format!("SDRS-HALF Position {case}"),
            None,
        )];
        match ctx
            .c
            .save_credit_note(&SaveCreditNote::new(draft, positions))
            .await
        {
            Ok(saved) => {
                let id = saved.credit_note.id.get();
                eprintln!("CREATED CreditNote {id} (case {case}, marker {note})");
                created.push((case, id, note));
            }
            Err(e) => {
                // The write may have happened: record, re-read, never retry.
                eprintln!("HALF-FAILED\tsave_credit_note case {case} failed: {e}");
                let found = drafts_by_marker(&ctx, &ctx.marker, "half_cent_reread").await;
                eprintln!(
                    "HALF-FAILED\tre-read by marker {} found ids {found:?}; stopping",
                    ctx.marker
                );
                break;
            }
        }
    }

    // Read back raw, record the sums, print the verdict.
    for (case, id, _) in &created {
        let Some(s) = read_sums_at(&ctx, "CreditNote", *id, &format!("half_cent_{case}")).await
        else {
            continue;
        };
        eprintln!(
            "HALF\t{case}\t{id}\t{}\t{}\t{}\t{}",
            s.net,
            s.tax,
            s.gross,
            s.positions_text()
        );
        let (_, _, half_up, half_even) = cases.iter().find(|c| c.0 == *case).unwrap();
        let head_tax = thousandths(&s.tax);
        let pos_tax = s.positions.first().and_then(|p| thousandths(&p[1]));
        let matches = |expect: &str| {
            let e = thousandths(expect);
            head_tax == e && pos_tax == e
        };
        let discriminates = half_up != half_even;
        eprintln!(
            "HALF-VERDICT\t{case}\thalf-up expects {half_up}, half-even expects {half_even} \
             (0.285 -> 0.28 half-even because 8 is even; 0.095 -> 0.10 half-even because 9 is odd, \
             so only case a discriminates)\tmeasured head tax {} / position tax {}\t\
             matches half-up={} matches half-even={} discriminating={discriminates}",
            s.tax,
            s.positions.first().map_or("-", |p| p[1].as_str()),
            matches(half_up),
            matches(half_even),
        );
    }

    // Delete only what this run created (own id, own marker, status 100).
    let mut undeleted: Vec<i64> = Vec::new();
    for (case, id, note) in &created {
        let (status, text) = ctx
            .raw(Method::GET, &format!("/CreditNote/{id}"), None, None)
            .await;
        let head = if status < 300 {
            raw_one(&text)
        } else {
            Value::Null
        };
        let own = text_of(&head, "id") == id.to_string()
            && text_of(&head, "customerInternalNote") == *note
            && text_of(&head, "status") == "100";
        if !own {
            eprintln!(
                "UNDELETED\tcase {case} id {id}: read-back is not own marker/status 100, not deleting"
            );
            undeleted.push(*id);
            continue;
        }
        match ctx.c.delete_credit_note(CreditNoteId::new(*id)).await {
            Ok(()) => eprintln!("DELETED\tCreditNote {id} (case {case})"),
            Err(e) => {
                eprintln!("UNDELETED\tcase {case} id {id}: delete failed: {e}; not retrying");
                undeleted.push(*id);
            }
        }
    }

    // Confirm they are gone.
    for (case, id, _) in &created {
        let (status, text) = ctx
            .raw(
                Method::GET,
                &format!("/CreditNote/{id}"),
                None,
                Some(&format!("half_cent_{case}_after_delete")),
            )
            .await;
        let gone = status == 404
            || (status < 300 && matches!(raw_objects(&text), Value::Null))
            || (status < 300 && raw_objects(&text).as_array().is_some_and(Vec::is_empty));
        eprintln!(
            "{}\tCreditNote {id} (case {case}) HTTP {status}",
            if gone { "GONE" } else { "STILL-THERE" }
        );
    }
    let left = drafts_by_marker(&ctx, &ctx.marker, "half_cent_list_after_delete").await;
    eprintln!(
        "GONE-LIST\tcredit notes with marker {} after delete: {left:?}",
        ctx.marker
    );
    let (seq_after, number_after) = credit_sequence(&ctx, "half_cent_sequence_after").await;
    eprintln!(
        "SEQUENCE\t{number_before} -> {number_after}\traw unchanged={}",
        seq_before == seq_after
    );
    for id in &undeleted {
        eprintln!("UNDELETED-DRAFT\tCreditNote {id} must be reported to the coordinator");
    }
    assert!(
        undeleted.is_empty(),
        "drafts left on the test account: {undeleted:?}"
    );
    assert_eq!(created.len(), cases.len(), "not every draft was saved");
}

/// Gross needed on the reference invoice for `case3`, in thousandths: the
/// largest single draft (49.50 net + 9.41 VAT = 58.91); the drafts are saved
/// and deleted one at a time, so only one counts against the cap.
const CASE3_NEEDED_GROSS_THOUSANDTHS: i64 = 58_910;

/// `(net in cents, tax rate in basis points)` of the `case3` drafts.
const CASE3_CASES: [(i64, i64); 6] = [
    (250, 1900),
    (1050, 1900),
    (4250, 1900),
    (4950, 1900),
    (1450, 700),
    (3050, 700),
];

/// VAT in cents of `net` cents at `rate_bp` basis points, rounded half away
/// from zero (`half_even == false`) or half to even (`true`); exact integers.
fn vat_cents(net: i64, rate_bp: i64, half_even: bool) -> i64 {
    let num = net * rate_bp;
    let (q, r) = (num / 10_000, num % 10_000);
    // `r` has the sign of `num`; `|r| * 2` against the divisor decides.
    let twice = r.abs() * 2;
    let away = if num < 0 { -1 } else { 1 };
    if twice > 10_000 || (twice == 10_000 && !(half_even && q % 2 == 0)) {
        q + away
    } else {
        q
    }
}

#[test]
fn vat_cents_rounds_half_up_and_half_even_exactly() {
    // (net, rate, half-up, half-even): the table of `case3`.
    for (net, rate, up, even) in [
        (250, 1900, 48, 48),
        (1050, 1900, 200, 200),
        (4250, 1900, 808, 808),
        (4950, 1900, 941, 940),
        (1450, 700, 102, 102),
        (3050, 700, 214, 214),
        (150, 1900, 29, 28),
        (-150, 1900, -29, -28),
    ] {
        assert_eq!(vat_cents(net, rate, false), up, "{net} {rate} half-up");
        assert_eq!(vat_cents(net, rate, true), even, "{net} {rate} half-even");
    }
}

fn cents_text(c: i64) -> String {
    format!(
        "{}{}.{:02}",
        if c < 0 { "-" } else { "" },
        c.abs() / 100,
        c.abs() % 100
    )
}

/// Deletes a credit note draft of this run (own id, own marker, status 100)
/// and confirms it is gone. `false` when anything about that failed.
async fn delete_own_draft(ctx: &Ctx, id: i64, note: &str, name: &str) -> bool {
    let (status, text) = ctx
        .raw(Method::GET, &format!("/CreditNote/{id}"), None, None)
        .await;
    let head = if status < 300 {
        raw_one(&text)
    } else {
        Value::Null
    };
    let own = text_of(&head, "id") == id.to_string()
        && text_of(&head, "customerInternalNote") == note
        && text_of(&head, "status") == "100";
    if !own {
        eprintln!("UNDELETED {id}\tread-back is not own marker/status 100, not deleting");
        return false;
    }
    if let Err(e) = ctx.c.delete_credit_note(CreditNoteId::new(id)).await {
        eprintln!("UNDELETED {id}\tdelete failed: {e}; not retrying");
        return false;
    }
    let (status, text) = ctx
        .raw(
            Method::GET,
            &format!("/CreditNote/{id}"),
            None,
            Some(&format!("{name}_after_delete")),
        )
        .await;
    let gone = status == 404
        || (status < 300 && matches!(raw_objects(&text), Value::Null))
        || (status < 300 && raw_objects(&text).as_array().is_some_and(Vec::is_empty));
    eprintln!(
        "{}\tCreditNote {id} HTTP {status}",
        if gone { "GONE" } else { "STILL-THERE" }
    );
    if !gone {
        eprintln!("UNDELETED {id}\tstill readable after delete");
    }
    gone
}

#[tokio::test]
#[ignore = "writes and deletes credit note drafts on the sevDesk test account"]
async fn case3_half_cent_float_check() {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping the half cent float check");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Ctx::new(&token, format!("SDRS-HALF3-{ts}"));
    verify_test_account(&ctx).await;

    // Read-only inputs.
    let methods = ctx.c.list_payment_methods().await.unwrap();
    let pm = methods
        .iter()
        .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
        .expect("a payment method with electronicInvoiceId 68")
        .id;
    let customer = existing_customer(&ctx).await;
    let person = contact_person(&ctx).await;
    let invoice_id = enshrined_invoice(&ctx, CASE3_NEEDED_GROSS_THOUSANDTHS).await;
    let (seq_before, number_before) =
        credit_sequence(&ctx, "half_cent_float_sequence_before").await;
    eprintln!(
        "SETUP contact {} payment method {} ref invoice {} next CN {number_before}",
        customer.contact.id.get(),
        pm.get(),
        invoice_id.get()
    );

    let mut undeleted: Vec<i64> = Vec::new();
    // (verdict, separates half-up from half-even) per processed case.
    let mut results: Vec<(String, bool)> = Vec::new();
    for (net, rate_bp) in CASE3_CASES {
        let net_text = cents_text(net);
        let rate_text = format!("{}", rate_bp / 100);
        let name = format!("half_cent_float_{net_text}_{rate_text}");
        let note = format!("{}-{net_text}-{rate_text}", ctx.marker);
        let half_up = vat_cents(net, rate_bp, false);
        let half_even = vat_cents(net, rate_bp, true);
        eprintln!(
            "EXPECT\t{net_text}\t{rate_text}\thalf-up {}\thalf-even {}\tseparates={}",
            cents_text(half_up),
            cents_text(half_even),
            half_up != half_even
        );
        let mut draft = CreditNoteDraft {
            credit_note_number: number_before.clone(),
            contact: ObjectRef::new(customer.contact.id),
            contact_person: ObjectRef::new(person),
            credit_note_date: date!(2026 - 10 - 01),
            delivery_date: datetime!(2026-10-01 0:00 +2),
            header: note.clone(),
            tax_rule: ObjectRef::new(TaxRuleId::new(1)),
            tax_text: format!("Umsatzsteuer {rate_text}%"),
            booking_category: CreditNoteCategory::Underachievement,
            currency: CurrencyCode::EUR,
            show_net: true,
            customer_internal_note: note.clone(),
            options: Default::default(),
        };
        draft.options.delivery_date_until = Some(datetime!(2026-10-31 0:00 +1));
        draft.options.ref_src_invoice = Some(ObjectRef::new(invoice_id));
        draft.options.address_name = Some(customer.name.clone());
        draft.options.address_street = Some(customer.street.into());
        draft.options.address_zip = Some(customer.zip.into());
        draft.options.address_city = Some(customer.city.into());
        draft.options.address_country = Some(ObjectRef::new(GERMANY));
        draft.options.payment_method = Some(ObjectRef::new(pm));
        draft.options.is_e_invoice = true;
        let mut position =
            net_position(1, net, &format!("SDRS-HALF3 {net_text} {rate_text}"), None);
        position.tax_rate_bp = rate_bp;
        let id = match ctx
            .c
            .save_credit_note(&SaveCreditNote::new(draft, vec![position]))
            .await
        {
            Ok(saved) => saved.credit_note.id.get(),
            Err(e) => {
                // The write may have happened: look by marker, never retry.
                eprintln!("HALF3-REFUSED\t{net_text}\t{rate_text}\tsave_credit_note failed: {e}");
                let found = drafts_by_marker(&ctx, &note, &format!("{name}_reread")).await;
                if !found.is_empty() {
                    for id in &found {
                        eprintln!("UNDELETED {id}\tfound by marker after a failed save");
                    }
                    break;
                }
                if rate_bp == 1900 {
                    eprintln!("HALF3-FAILED\ta 19 % case was refused; stopping");
                    break;
                }
                eprintln!("HALF3-REFUSED\tnothing written for this case; continuing");
                results.push(("refused".to_owned(), half_up != half_even));
                continue;
            }
        };
        eprintln!("CREATED CreditNote {id} (marker {note})");
        let sums = read_sums_at(&ctx, "CreditNote", id, &name).await;
        if let Some(s) = &sums {
            let want = half_up * 10;
            let head = thousandths(&s.tax);
            let pos = s.positions.first().and_then(|p| thousandths(&p[1]));
            let verdict = if head == Some(want) && pos == Some(want) {
                "exact-half-up"
            } else if head == Some(want - 10) && pos == Some(want - 10) {
                "one-cent-lower"
            } else {
                "other"
            };
            let pos_text = s
                .positions
                .first()
                .map_or("-".to_owned(), |p| format!("{}/{}", p[1], p[2]));
            eprintln!(
                "HALF3\t{net_text}\t{rate_text}\t{id}\t{}\t{}\t{pos_text}\t{}\t{verdict}",
                s.tax,
                s.gross,
                cents_text(half_up)
            );
            results.push((verdict.to_owned(), half_up != half_even));
        } else {
            results.push(("unreadable".to_owned(), half_up != half_even));
        }
        // Delete this draft before the next one is saved.
        if !delete_own_draft(&ctx, id, &note, &name).await {
            undeleted.push(id);
            eprintln!("HALF3-FAILED\tstopping: no further drafts after an undeleted one");
            break;
        }
    }

    let left = drafts_by_marker(&ctx, &ctx.marker, "half_cent_float_list_after_delete").await;
    eprintln!(
        "GONE-LIST\tcredit notes with marker {} after delete: {left:?}",
        ctx.marker
    );
    let (seq_after, number_after) = credit_sequence(&ctx, "half_cent_float_sequence_after").await;
    eprintln!(
        "SEQUENCE\t{number_before} -> {number_after}\traw unchanged={}",
        seq_before == seq_after
    );
    let count = |v: &str, sep: bool| results.iter().filter(|r| r.0 == v && r.1 == sep).count();
    eprintln!(
        "HALF3-SUMMARY\tcases processed {} of {}\texact-half-up {} (separating {}), \
         one-cent-lower {} (separating {}), other {}, refused {}, unreadable {}",
        results.len(),
        CASE3_CASES.len(),
        count("exact-half-up", false) + count("exact-half-up", true),
        count("exact-half-up", true),
        count("one-cent-lower", false) + count("one-cent-lower", true),
        count("one-cent-lower", true),
        count("other", false) + count("other", true),
        count("refused", false) + count("refused", true),
        count("unreadable", false) + count("unreadable", true),
    );
    for id in &undeleted {
        eprintln!("UNDELETED-DRAFT\tCreditNote {id} must be reported to the coordinator");
    }
    assert!(
        undeleted.is_empty() && left.is_empty(),
        "drafts left on the test account: {undeleted:?} {left:?}"
    );
}
