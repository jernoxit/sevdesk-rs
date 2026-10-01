//! The credit note part of the B2B chain: save, render, enshrine, XML, and what
//! the credit note does to cancelling its invoice.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helper code outside #[test] functions: a failure here is a test failure"
)]

use super::docs::*;
use super::harness::*;
use reqwest::Method;
use sevdesk::*;
use time::macros::{date, datetime};

fn draft(
    number: String,
    invoice: &Invoice,
    spec: &InvoiceSpec<'_>,
    note: String,
    is_e_invoice: bool,
    payment_method: PaymentMethodId,
) -> CreditNoteDraft {
    CreditNoteDraft {
        credit_note_number: number,
        contact: ObjectRef::new(spec.customer.contact.id),
        contact_person: ObjectRef::new(spec.person),
        credit_note_date: date!(2026 - 10 - 01),
        delivery_date: datetime!(2026-10-01 0:00 +2),
        delivery_date_until: Some(datetime!(2026-10-31 0:00 +1)),
        header: note.clone(),
        head_text: None,
        foot_text: None,
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "Umsatzsteuer 19%".into(),
        booking_category: CreditNoteCategory::Underachievement,
        ref_src_invoice: Some(ObjectRef::new(invoice.id)),
        currency: CurrencyCode::EUR,
        show_net: true,
        customer_internal_note: note,
        address_name: Some(spec.customer.name.clone()),
        address_street: Some(spec.customer.street.into()),
        address_zip: Some(spec.customer.zip.into()),
        address_city: Some(spec.customer.city.into()),
        address_country: Some(ObjectRef::new(GERMANY)),
        payment_method: is_e_invoice.then(|| ObjectRef::new(payment_method)),
        is_e_invoice,
    }
}

/// `invoice` is the enshrined invoice of the chain (net 55.00).
pub(crate) async fn credit_note_on_enshrined_invoice(
    ctx: &Ctx,
    invoice: &Invoice,
    spec: &InvoiceSpec<'_>,
) {
    let payment_method = spec
        .payment_method
        .expect("an e-invoice credit note needs one");
    let note = format!("{}-CN", ctx.marker);
    let number = ctx.c.next_credit_note_number().await.unwrap();
    let save = SaveCreditNote::new(
        draft(
            number.clone(),
            invoice,
            spec,
            note.clone(),
            true,
            payment_method,
        ),
        vec![net_position(1, 1000, "SDRS-LIVE Teilgutschrift", None)],
    );
    let saved: SavedCreditNote = objects(&ok(ctx
        .raw(
            Method::POST,
            "/CreditNote/Factory/saveCreditNote",
            Some(json(&save)),
            Some("write_credit_note_save"),
        )
        .await));
    let cn = saved.credit_note;
    eprintln!("CREATED credit note {} number {number}", cn.id);
    assert_eq!(cn.status, Some(CreditNoteStatus::Draft));
    assert_eq!(cn.currency, CurrencyCode::EUR);
    assert_eq!(cn.credit_note_number.as_deref(), Some(number.as_str()));
    assert_eq!(cn.sum_gross, Some(eur(1190)));
    assert_eq!(cn.ref_src_invoice.map(|r| r.id), Some(invoice.id));
    assert!(cn.enshrined.is_none());

    let pdf = ctx.c.credit_note_pdf(cn.id).await.unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF"));
    measured(&format!(
        "credit note draft getPdf: filename {:?}, {} bytes",
        pdf.filename,
        pdf.bytes.len()
    ));
    let after_pdf = ctx.c.credit_note(cn.id).await.unwrap();
    assert_eq!(
        after_pdf.status,
        Some(CreditNoteStatus::Draft),
        "preventSendBy"
    );

    let sent = ctx
        .c
        .send_credit_note_by(cn.id, &SendBy::ENSHRINE)
        .await
        .unwrap();
    assert_eq!(sent.status, Some(CreditNoteStatus::Open));
    assert!(sent.enshrined.is_some());
    assert_eq!(sent.currency, CurrencyCode::EUR);
    let xml = ctx.c.credit_note_xml(cn.id).await.unwrap();
    assert!(xml.trim_start().starts_with('<'));
    assert!(
        xml.contains("CreditNote") || xml.contains("381"),
        "not a credit note XML"
    );
    let listed = ctx
        .c
        .list_credit_notes(&CreditNoteQuery {
            customer_internal_note: Some(note.clone()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, cn.id);

    // The credit note stands beside the invoice now.
    let err = ctx.c.cancel_invoice(invoice.id).await.unwrap_err();
    assert!(
        matches!(&err, SevdeskError::Api { status: 422, .. }),
        "{err}"
    );
    assert_eq!(err.known_code(), Some(KnownCode::CreditNoteExists), "{err}");

    // A second, draft credit note on the same invoice deletes with 200.
    let number = ctx.c.next_credit_note_number().await.unwrap();
    let draft_note = format!("{}-CN-DRAFT", ctx.marker);
    let saved = ctx
        .c
        .save_credit_note(&SaveCreditNote::new(
            draft(
                number.clone(),
                invoice,
                spec,
                draft_note,
                false,
                payment_method,
            ),
            vec![net_position(1, 500, "SDRS-LIVE Entwurf", None)],
        ))
        .await
        .unwrap();
    eprintln!(
        "CREATED credit note draft {} number {number}",
        saved.credit_note.id
    );
    ctx.c
        .delete_credit_note(saved.credit_note.id)
        .await
        .unwrap();
    let gone = ctx.c.credit_note(saved.credit_note.id).await.unwrap_err();
    assert!(matches!(gone, SevdeskError::NotFound { .. }), "{gone}");
}
