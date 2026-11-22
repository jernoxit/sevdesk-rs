//! Part 1, documents: the invoice draft, the credit note draft and the voucher
//! draft, each created with every option set (and the voucher updated through
//! the save factory with its ids).

use crate::Gate;
use crate::gate::expect_keys;
use crate::harness::eur;
use crate::setup::CREDIT_NET;
use sevdesk::*;
use time::macros::{date, datetime};

const POSITION_KEYS: [&str; 7] = [
    "quantity",
    "price",
    "name",
    "text",
    "unity",
    "taxRate",
    "positionNumber",
];

/// A position with every `PositionOptions` field set.
fn position(marker: &str, quantity: i64, minor: i64) -> DocumentPositionDraft {
    let mut p = DocumentPositionDraft {
        quantity,
        price: eur(minor),
        name: format!("{marker} position"),
        unity: PIECE,
        tax_rate_bp: 1900,
        options: Default::default(),
    };
    p.options.text = Some("Gate position text".into());
    p.options.position_number = Some(7);
    p
}

#[allow(
    deprecated,
    reason = "the gate sets the deprecated options on purpose: it proves their encoding and that a draft stays a draft"
)]
fn set_deprecated_invoice_options(o: &mut InvoiceOptions, number: String) {
    o.send_date = Some(date!(2026 - 10 - 05));
    o.send_type = Some(SendType::Vpdf);
    o.invoice_number = Some(number);
}

#[allow(
    deprecated,
    reason = "the gate sets the deprecated options on purpose: it proves their encoding and that a draft stays a draft"
)]
fn set_deprecated_credit_note_options(o: &mut CreditNoteOptions) {
    o.send_date = Some(date!(2026 - 10 - 06));
    o.send_type = Some(SendType::Vpdf);
}

/// The invoice draft with every option; returns the head as read back and its
/// number.
pub(crate) async fn invoice(g: &mut Gate<'_>) -> (serde_json::Value, String) {
    let ctx = g.ctx;
    let n = ctx.marker.clone();
    let (contact, person, pm) = (g.inp.customer.contact.id, g.inp.person, g.inp.pm);
    let custom_number = format!("{n}-INV");
    let mut draft = InvoiceDraft {
        contact: ObjectRef::new(contact),
        contact_person: ObjectRef::new(person),
        invoice_date: date!(2026 - 10 - 03),
        delivery_date: datetime!(2026-10-03 0:00 +2),
        header: format!("{n}-invoice-header"),
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "Umsatzsteuer 19% (gate)".into(),
        currency: CurrencyCode::EUR,
        show_net: true,
        time_to_pay: 17,
        customer_internal_note: format!("{n}-invoice"),
        options: Default::default(),
    };
    let o = &mut draft.options;
    o.delivery_date_until = Some(datetime!(2026-10-28 0:00 +1));
    o.head_text = Some("Gate head text".into());
    o.foot_text = Some("Gate foot text".into());
    o.address_name = Some(format!("Gate Invoice Name {n}"));
    o.address_street = Some("Gatestrasse 11".into());
    o.address_zip = Some("10999".into());
    o.address_city = Some("Gateinvoicecity".into());
    o.address_country = Some(ObjectRef::new(GERMANY));
    o.payment_method = Some(ObjectRef::new(pm));
    o.is_e_invoice = true;
    o.address = Some("Gate Address Line 1\nGate Address Line 2".into());
    o.small_settlement = Some(true);
    o.pay_date = Some(date!(2026 - 10 - 09));
    set_deprecated_invoice_options(o, custom_number.clone());
    let save = SaveInvoice::new(draft, vec![position(&n, 3, 1234)]);
    let sent = serde_json::to_value(&save).unwrap();
    expect_keys(
        "save_invoice",
        &sent["invoice"],
        &[
            "contact",
            "contactPerson",
            "invoiceDate",
            "deliveryDate",
            "deliveryDateUntil",
            "header",
            "headText",
            "footText",
            "discount",
            "status",
            "taxRule",
            "taxRate",
            "taxText",
            "invoiceType",
            "currency",
            "showNet",
            "timeToPay",
            "customerInternalNote",
            "addressName",
            "addressStreet",
            "addressZip",
            "addressCity",
            "addressCountry",
            "paymentMethod",
            "propertyIsEInvoice",
            "address",
            "smallSettlement",
            "sendDate",
            "payDate",
            "sendType",
            "invoiceNumber",
        ],
        &[],
    );
    expect_keys(
        "save_invoice/pos",
        &sent["invoicePosSave"][0],
        &POSITION_KEYS,
        &[],
    );

    g.may_create();
    let saved = match ctx.c.save_invoice(&save).await {
        Ok(saved) => saved,
        Err(e) => g.create_failed("save_invoice", &e),
    };
    let id = saved.invoice.id.get();
    g.created("Invoice", id);
    eprintln!(
        "NUMBER Invoice {id} invoiceNumber={:?}",
        saved.invoice.invoice_number
    );
    let head = g
        .check(
            "invoice",
            "save_invoice",
            &sent["invoice"],
            None,
            &format!("/Invoice/{id}"),
            false,
        )
        .await;
    g.check_pos(
        "invoice",
        "save_invoice/pos",
        &sent["invoicePosSave"][0],
        None,
        &format!("/InvoicePos?invoice[id]={id}&invoice[objectName]=Invoice"),
    )
    .await;
    let number = head["invoiceNumber"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    g.number_check("Invoice", "RE", &number, &custom_number)
        .await;
    g.rec.flush("invoice");
    (head, number)
}

/// The credit note draft with every option; returns the head as read back.
pub(crate) async fn credit_note(g: &mut Gate<'_>) -> serde_json::Value {
    let ctx = g.ctx;
    let n = ctx.marker.clone();
    let (person, pm) = (g.inp.person, g.inp.pm);
    let origin_id = g.inp.origin.id;
    let origin_contact = g
        .inp
        .origin
        .contact
        .expect("the origin invoice names a contact")
        .id;
    let number = match ctx.c.next_credit_note_number().await {
        Ok(number) => number,
        Err(e) => panic!("next_credit_note_number failed: {e}"),
    };
    let origin_until = g.inp.origin.delivery_date_until;
    let mut draft = CreditNoteDraft {
        credit_note_number: number.clone(),
        contact: ObjectRef::new(origin_contact),
        contact_person: ObjectRef::new(person),
        credit_note_date: date!(2026 - 10 - 04),
        delivery_date: datetime!(2026-10-04 0:00 +2),
        header: format!("{n}-credit-note-header"),
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "Umsatzsteuer 19% (gate)".into(),
        booking_category: CreditNoteCategory::Underachievement,
        currency: CurrencyCode::EUR,
        show_net: true,
        customer_internal_note: format!("{n}-credit-note"),
        options: Default::default(),
    };
    let o = &mut draft.options;
    // The source invoice's own value: sevDesk refuses it (422, 463) when the
    // source has none.
    o.delivery_date_until = origin_until;
    o.head_text = Some("Gate credit head text".into());
    o.foot_text = Some("Gate credit foot text".into());
    o.ref_src_invoice = Some(ObjectRef::new(origin_id));
    o.address_name = Some(format!("Gate Credit Name {n}"));
    o.address_street = Some("Gatecreditstrasse 12".into());
    o.address_zip = Some("10998".into());
    o.address_city = Some("Gatecreditcity".into());
    o.address_country = Some(ObjectRef::new(GERMANY));
    o.payment_method = Some(ObjectRef::new(pm));
    o.is_e_invoice = true;
    o.address = Some("Gate Credit Line 1\nGate Credit Line 2".into());
    o.small_settlement = Some(true);
    set_deprecated_credit_note_options(o);
    let save = SaveCreditNote::new(draft, vec![position(&n, 1, CREDIT_NET)]);
    let sent = serde_json::to_value(&save).unwrap();
    expect_keys(
        "save_credit_note",
        &sent["creditNote"],
        &[
            "creditNoteNumber",
            "contact",
            "contactPerson",
            "creditNoteDate",
            "deliveryDate",
            "deliveryDateUntil",
            "header",
            "headText",
            "footText",
            "discount",
            "status",
            "taxRule",
            "taxRate",
            "taxText",
            "creditNoteType",
            "bookingCategory",
            "refSrcInvoice",
            "currency",
            "showNet",
            "customerInternalNote",
            "addressName",
            "addressStreet",
            "addressZip",
            "addressCity",
            "addressCountry",
            "paymentMethod",
            "propertyIsEInvoice",
            "address",
            "smallSettlement",
            "sendDate",
            "sendType",
        ],
        // No source invoice with a `deliveryDateUntil` in the account: left
        // unset, reported as UNTESTED.
        if origin_until.is_none() {
            &["deliveryDateUntil"]
        } else {
            &[]
        },
    );
    expect_keys(
        "save_credit_note/pos",
        &sent["creditNotePosSave"][0],
        &POSITION_KEYS,
        &[],
    );

    g.may_create();
    let saved = match ctx.c.save_credit_note(&save).await {
        Ok(saved) => saved,
        Err(e) => g.create_failed("save_credit_note", &e),
    };
    let id = saved.credit_note.id.get();
    g.created("CreditNote", id);
    eprintln!(
        "NUMBER CreditNote {id} creditNoteNumber={:?}",
        saved.credit_note.credit_note_number
    );
    let head = g
        .check(
            "credit_note",
            "save_credit_note",
            &sent["creditNote"],
            None,
            &format!("/CreditNote/{id}"),
            false,
        )
        .await;
    g.check_pos(
        "credit_note",
        "save_credit_note/pos",
        &sent["creditNotePosSave"][0],
        None,
        &format!("/CreditNotePos?creditNote[id]={id}&creditNote[objectName]=CreditNote"),
    )
    .await;
    let read_back = head["creditNoteNumber"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    g.number_check("CreditNote", "CN", &read_back, &number)
        .await;
    g.rec.flush("credit_note");
    head
}

/// The voucher draft with every option and a gross position.
fn voucher_save(
    g: &Gate<'_>,
    update: Option<(VoucherId, VoucherPositionId)>,
    suffix: &str,
    gross_minor: i64,
) -> SaveVoucher {
    let n = &g.ctx.marker;
    let mut v = VoucherDraft::new(
        date!(2026 - 10 - 02),
        format!("{n} Supplier{suffix}"),
        format!("{n}-voucher{suffix}"),
        TaxRuleId::new(14),
        CurrencyCode::EUR,
    );
    let o = &mut v.options;
    o.id = update.map(|(id, _)| id);
    o.delivery_date = Some(date!(2026 - 10 - 01));
    o.delivery_date_until = Some(date!(2026 - 10 - 02));
    o.supplier = Some(ObjectRef::new(g.inp.customer.contact.id));
    o.payment_deadline = Some(if suffix.is_empty() {
        date!(2026 - 10 - 15)
    } else {
        date!(2026 - 10 - 16)
    });
    o.document = g.inp.document.map(ObjectRef::new);
    o.pay_date = Some(if suffix.is_empty() {
        date!(2026 - 10 - 10)
    } else {
        date!(2026 - 10 - 11)
    });
    let mut p = VoucherPositionDraft::new(
        AccountDatevId::new(4285),
        0,
        eur(gross_minor - 500),
        format!("{n} voucher position{suffix}"),
    );
    let po = &mut p.options;
    po.id = update.map(|(_, pos)| pos);
    po.accounting_type = g.inp.accounting_type.map(ObjectRef::new);
    po.is_asset = Some(true);
    po.sum_gross = Some(eur(gross_minor));
    SaveVoucher {
        voucher: v,
        positions: vec![p],
        filename: None,
    }
}

const VOUCHER_KEYS: [&str; 14] = [
    "voucherDate",
    "deliveryDate",
    "deliveryDateUntil",
    "supplierName",
    "supplier",
    "description",
    "status",
    "creditDebit",
    "voucherType",
    "taxRule",
    "currency",
    "paymentDeadline",
    "document",
    "payDate",
];

const VOUCHER_POS_KEYS: [&str; 8] = [
    "accountDatev",
    "taxRate",
    "net",
    "sumNet",
    "comment",
    "accountingType",
    "isAsset",
    "sumGross",
];

/// The voucher draft with every option, then updated through the save factory
/// with the voucher and position ids. Returns the head and the position as read
/// back after the update.
pub(crate) async fn voucher(g: &mut Gate<'_>) -> (serde_json::Value, serde_json::Value) {
    let ctx = g.ctx;
    let save = voucher_save(g, None, "", 1500);
    let sent = serde_json::to_value(&save).unwrap();
    expect_keys(
        "save_voucher/create",
        &sent["voucher"],
        &VOUCHER_KEYS,
        &["document"],
    );
    expect_keys(
        "save_voucher/create/pos",
        &sent["voucherPosSave"][0],
        &VOUCHER_POS_KEYS,
        &["accountingType"],
    );

    g.may_create();
    let saved = match ctx.c.save_voucher(&save).await {
        Ok(saved) => saved,
        Err(e) => g.create_failed("save_voucher", &e),
    };
    let id = saved.voucher.id;
    g.created("Voucher", id.get());
    let pos_id = saved.voucher_pos.first().expect("a saved position").id;
    let head_path = format!("/Voucher/{id}");
    let pos_path = format!("/VoucherPos?voucher[id]={id}&voucher[objectName]=Voucher");
    let head = g
        .check(
            "voucher",
            "save_voucher/create",
            &sent["voucher"],
            None,
            &head_path,
            false,
        )
        .await;
    let pos = g
        .check_pos(
            "voucher",
            "save_voucher/create/pos",
            &sent["voucherPosSave"][0],
            None,
            &pos_path,
        )
        .await;
    gross_note(
        g,
        "save_voucher/create",
        &sent["voucherPosSave"][0],
        &pos,
        &head,
    );

    // Update through the factory: `VoucherOptions::id` and
    // `VoucherPositionOptions::id`, every position sent with its id.
    let save = voucher_save(g, Some((id, pos_id)), "-U", 1600);
    let sent_u = serde_json::to_value(&save).unwrap();
    expect_keys(
        "save_voucher/update",
        &sent_u["voucher"],
        &VOUCHER_KEYS,
        &["document"],
    );
    let result = ctx.c.save_voucher(&save).await;
    if let Ok(saved) = &result {
        assert_eq!(
            saved.voucher.id, id,
            "the update must answer the same voucher"
        );
    }
    let head_u = g
        .update_step(
            "voucher",
            "save_voucher/update",
            sent_u["voucher"].clone(),
            &head,
            &head_path,
            result,
        )
        .await;
    let list = g.get_list(&pos_path).await;
    assert_eq!(list.len(), 1, "the update must not add a position");
    crate::gate::compare(
        &mut g.rec,
        "voucher",
        "save_voucher/update/pos",
        &sent_u["voucherPosSave"][0],
        Some(&pos),
        &list[0],
        false,
    );
    gross_note(
        g,
        "save_voucher/update",
        &sent_u["voucherPosSave"][0],
        &list[0],
        &head_u,
    );
    g.rec.flush("voucher");
    (head_u, list[0].clone())
}

/// Records what sevDesk does with `net=false`, `sumGross` and the always-sent
/// `sumNet`.
fn gross_note(
    g: &mut Gate<'_>,
    route: &str,
    sent: &serde_json::Value,
    pos: &serde_json::Value,
    head: &serde_json::Value,
) {
    let show = |v: &serde_json::Value, k: &str| v[k].to_string();
    let text = format!(
        "sent net={} sumNet={} sumGross={}; position read back net={} sumNet={} sumGross={}; voucher read back sumNet={} sumGross={}",
        show(sent, "net"),
        show(sent, "sumNet"),
        show(sent, "sumGross"),
        show(pos, "net"),
        show(pos, "sumNet"),
        show(pos, "sumGross"),
        show(head, "sumNet"),
        show(head, "sumGross"),
    );
    eprintln!("VOUCHER-GROSS\t{route}\t{text}");
    g.rec.note("voucher", route, "net+sumNet+sumGross", &text);
}
