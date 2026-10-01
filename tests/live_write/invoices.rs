//! Invoice writes: the B2B e-invoice chain with its credit note, and the B2C
//! cancel, delete and list paths.

use super::credit_notes::credit_note_on_enshrined_invoice;
use super::docs::*;
use super::harness::*;
use reqwest::Method;
use sevdesk::*;
use time::OffsetDateTime;

fn api_error(e: &SevdeskError, status: u16, known: KnownCode) {
    assert!(
        matches!(e, SevdeskError::Api { status: s, .. } if *s == status),
        "{e}"
    );
    assert_eq!(e.known_code(), Some(known), "{e}");
}

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_21_b2b_einvoice_chain_with_credit_note() {
    run_docs("b2b", |ctx| async move {
        let methods = ok(ctx.raw(Method::GET, "/PaymentMethod", None, Some("payment_method_list_live")).await);
        let methods: Vec<PaymentMethod> = objects(&methods);
        let typed = ctx.c.list_payment_methods().await.unwrap();
        assert_eq!(typed.len(), methods.len());
        let online = methods
            .iter()
            .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
            .expect("a payment method with electronicInvoiceId 68");

        let customer = b2b_customer(&ctx, false).await;
        eprintln!(
            "CREATED contact {} (B2B) customer number {:?}",
            customer.contact.id, customer.contact.customer_number
        );
        let spec = InvoiceSpec {
            customer: &customer,
            person: contact_person(&ctx).await,
            note: ctx.marker.clone(),
            b2b: true,
            payment_method: Some(online.id),
            with_address: true,
        };
        let save = SaveInvoice::new(
            invoice_draft(&spec),
            vec![
                net_position(3, 1000, "SDRS-LIVE Beratung", None),
                net_position(1, 2500, "SDRS-LIVE Paket", Some("Zweite Zeile\nund eine dritte")),
            ],
        );
        let saved: SavedInvoice = objects(&ok(ctx
            .raw(
                Method::POST,
                "/Invoice/Factory/saveInvoice",
                Some(json(&save)),
                Some("write_invoice_save"),
            )
            .await));
        let inv = saved.invoice;
        eprintln!("CREATED invoice {} (draft, B2B)", inv.id);
        assert_eq!(inv.status, Some(InvoiceStatus::Draft));
        assert_eq!(inv.currency, CurrencyCode::EUR);
        assert_eq!(inv.sum_net, Some(eur(5500)));
        assert_eq!(inv.sum_gross, Some(eur(6545)));
        assert_eq!(inv.customer_internal_note.as_deref(), Some(ctx.marker.as_str()));
        assert!(inv.enshrined.is_none());
        measured(&format!("draft invoiceNumber: {:?}", inv.invoice_number));

        // Is the address on the document, and what else comes back on GET?
        let read = raw_one(&ok(ctx
            .raw(Method::GET, &format!("/Invoice/{}", inv.id), None, None)
            .await));
        let read = &read;
        assert_eq!(read["addressName"], customer.name.as_str());
        assert_eq!(read["addressStreet"], STREET);
        assert_eq!(read["addressZip"], ZIP);
        assert_eq!(read["addressCity"], CITY);
        measured(&format!(
            "GET Invoice: paymentMethod {}, propertyIsEInvoice {}, showNet {}, deliveryDateUntil {}, language {}",
            read["paymentMethod"], read["propertyIsEInvoice"], read["showNet"],
            read["deliveryDateUntil"], read["language"]
        ));

        // Measured: an e-invoice draft has no number, and rendering it fails.
        let draft_err = ctx.c.invoice_pdf(inv.id).await.unwrap_err();
        measured(&format!("e-invoice draft getPdf: {draft_err}"));
        assert!(matches!(draft_err, SevdeskError::Api { status: 503, .. }));
        let still = ctx.c.invoice(inv.id).await.unwrap();
        assert_eq!(still.status, Some(InvoiceStatus::Draft));
        assert!(still.invoice_number.is_none());

        // English, WITHOUT a render call, then enshrine.
        let english = DocumentParameter::Language(DocumentLanguage::EnUs);
        match ctx.c.change_invoice_parameter(inv.id, english).await {
            Ok(()) => measured("changeParameter language on an e-invoice draft: ok"),
            Err(e) => measured(&format!("changeParameter language on an e-invoice draft: {e}")),
        }
        let sent = ctx.c.send_invoice_by(inv.id, &SendBy::ENSHRINE).await.unwrap();
        match ctx.c.change_invoice_parameter(inv.id, english).await {
            Ok(()) => measured("changeParameter language on the enshrined e-invoice: ok"),
            Err(e) => measured(&format!("changeParameter language on the enshrined e-invoice: {e}")),
        }
        assert_eq!(sent.status, Some(InvoiceStatus::Open));
        assert_eq!(sent.currency, CurrencyCode::EUR);
        assert!(sent.enshrined.is_some());
        assert_eq!(sent.sum_gross, Some(eur(6545)));
        let number = sent.invoice_number.clone().expect("number after sendBy");
        eprintln!("ENSHRINED invoice {} number {number}", inv.id);

        let pdf_body = ok(ctx
            .raw(
                Method::GET,
                &format!("/Invoice/{}/getPdf?preventSendBy=true", inv.id),
                None,
                Some("invoice_pdf"),
            )
            .await);
        let pdf = raw_objects(&pdf_body);
        measured(&format!(
            "enshrined getPdf: filename {}, mime {}, content {} chars",
            pdf["filename"],
            pdf["mimetype"],
            pdf["content"].as_str().map_or(0, str::len)
        ));
        shorten_capture("invoice_pdf");
        let en = ctx.c.invoice_pdf(inv.id).await.unwrap();
        assert_eq!(en.filename, format!("{number}.pdf"));
        let en_path = keep_file("b2b_enshrined_en.pdf", &en.bytes);
        match pdf_text(&en_path) {
            Some(en) => {
                measured(&format!(
                    "enshrined PDF after en_US: mentions INVOICE {}, Rechnung {}",
                    en.to_uppercase().contains("INVOICE"),
                    en.contains("Rechnung"),
                ));
            }
            _ => measured("pdftotext missing: PDFs kept in target/tmp/captures for a manual look"),
        }

        let xml_body = ok(ctx
            .raw(Method::GET, &format!("/Invoice/{}/getXml", inv.id), None, Some("invoice_xml"))
            .await);
        shorten_capture("invoice_xml");
        assert!(objects::<String>(&xml_body).trim_start().starts_with('<'));
        let xml = ctx.c.invoice_xml(inv.id).await.unwrap();
        assert!(xml.contains("Invoice") || xml.contains("INVOICE"), "not a UBL/CII invoice");

        credit_note_on_enshrined_invoice(&ctx, &sent, &spec).await;
    })
    .await;
}

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_22_b2c_cancel_delete_and_list() {
    run_docs("b2c", |ctx| async move {
        let before = OffsetDateTime::now_utc() - time::Duration::seconds(5);
        let customer = b2c_customer(&ctx).await;
        eprintln!("CREATED contact {} (B2C)", customer.contact.id);
        let person = contact_person(&ctx).await;
        let spec = |note: String, with_address| InvoiceSpec {
            customer: &customer,
            person,
            note,
            b2b: false,
            payment_method: None,
            with_address,
        };
        let positions = || {
            vec![
                net_position(1, 1190, "SDRS-LIVE Einzelposition", None),
                net_position(2, 500, "SDRS-LIVE Doppelposition", None),
            ]
        };

        // No address fields: does sevDesk take the contact's address by itself?
        let saved = ctx
            .c
            .save_invoice(&SaveInvoice::new(
                invoice_draft(&spec(ctx.marker.clone(), false)),
                positions(),
            ))
            .await
            .unwrap();
        let original = saved.invoice.id;
        eprintln!("CREATED invoice {original} (B2C)");
        assert_eq!(saved.invoice.currency, CurrencyCode::EUR);
        assert_eq!(saved.invoice.sum_gross, Some(eur(2190)));
        let read = raw_one(&ok(ctx
            .raw(Method::GET, &format!("/Invoice/{original}"), None, None)
            .await));
        measured(&format!(
            "draft without address fields: addressName {}, addressStreet {}",
            read["addressName"], read["addressStreet"]
        ));
        // A plain draft renders, and stays a draft.
        let draft_pdf = ctx.c.invoice_pdf(original).await.unwrap();
        assert!(draft_pdf.bytes.starts_with(b"%PDF"));
        measured(&format!(
            "draft getPdf: filename {:?}, mime {}, {} bytes",
            draft_pdf.filename,
            draft_pdf.mime_type,
            draft_pdf.bytes.len()
        ));
        assert_eq!(draft_pdf.filename, ".pdf");
        let still = ctx.c.invoice(original).await.unwrap();
        assert_eq!(still.status, Some(InvoiceStatus::Draft));
        ctx.c
            .change_invoice_parameter(
                original,
                DocumentParameter::Language(DocumentLanguage::EnUs),
            )
            .await
            .unwrap();
        let sent = ctx
            .c
            .send_invoice_by(original, &SendBy::ENSHRINE)
            .await
            .unwrap();
        assert_eq!(sent.status, Some(InvoiceStatus::Open));
        let en = ctx.c.invoice_pdf(original).await.unwrap();
        let path = keep_file("b2c_enshrined_en.pdf", &en.bytes);
        if let Some(text) = pdf_text(&path) {
            measured(&format!(
                "B2C PDF after en_US before sendBy: INVOICE {}, Rechnung {}",
                text.to_uppercase().contains("INVOICE"),
                text.contains("Rechnung")
            ));
            assert!(
                text.to_uppercase().contains("INVOICE"),
                "PDF is not English"
            );
        }
        eprintln!(
            "ENSHRINED invoice {original} number {:?}",
            sent.invoice_number
        );

        let cancelled: Invoice = objects_one(&ok_status(
            201,
            ctx.raw(
                Method::POST,
                &format!("/Invoice/{original}/cancelInvoice"),
                None,
                Some("write_invoice_cancel"),
            )
            .await,
        ));
        assert_eq!(cancelled.currency, CurrencyCode::EUR);
        assert!(cancelled.cancels(original), "{cancelled:?}");
        eprintln!(
            "CREATED cancellation invoice {} number {:?}",
            cancelled.id, cancelled.invoice_number
        );
        let again = ctx.c.cancel_invoice(original).await.unwrap_err();
        api_error(&again, 422, KnownCode::AlreadyCancelled);
        let undeletable = ctx.c.delete_invoice(original).await.unwrap_err();
        api_error(&undeletable, 409, KnownCode::NotDeletable);

        // A draft deletes.
        let note = format!("{}-DRAFT", ctx.marker);
        let draft = ctx
            .c
            .save_invoice(&SaveInvoice::new(
                invoice_draft(&spec(note, true)),
                positions(),
            ))
            .await
            .unwrap()
            .invoice
            .id;
        eprintln!("CREATED invoice {draft} (draft, deleted again)");
        ctx.c.delete_invoice(draft).await.unwrap();
        let gone = ctx.c.invoice(draft).await.unwrap_err();
        assert!(matches!(gone, SevdeskError::NotFound { .. }), "{gone}");

        // Listing: updateAfter with a Unix time, and the exact note.
        let recent = ctx
            .c
            .list_invoices(&InvoiceQuery {
                update_after: Some(before),
                ..Default::default()
            })
            .await
            .unwrap();
        let ids: Vec<_> = recent.iter().map(|i| i.id).collect();
        measured(&format!(
            "updateAfter=<start of run>: {} invoices",
            ids.len()
        ));
        assert!(ids.contains(&original) && ids.contains(&cancelled.id));
        assert!(!ids.contains(&draft));
        let by_note = ctx
            .c
            .list_invoices(&InvoiceQuery {
                customer_internal_note: Some(ctx.marker.clone()),
                ..Default::default()
            })
            .await
            .unwrap();
        let mut by_note: Vec<_> = by_note.iter().map(|i| i.id).collect();
        by_note.sort();
        let mut expected = vec![original, cancelled.id];
        expected.sort();
        assert_eq!(by_note, expected, "original and cancellation invoice");
    })
    .await;
}

fn ok_status(want: u16, (status, text): (u16, String)) -> String {
    assert_eq!(
        status,
        want,
        "{}",
        text.chars().take(400).collect::<String>()
    );
    text
}
