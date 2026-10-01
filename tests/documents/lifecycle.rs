//! Finalizing, cancelling, deleting, rendering and sending documents.

use super::*;

#[tokio::test]
async fn resetting_an_invoice_or_credit_note_to_open_is_a_bodyless_put() {
    let server = MockServer::start().await;
    for document in ["Invoice/5", "CreditNote/6"] {
        Mock::given(method("PUT"))
            .and(path(format!("/{document}/resetToOpen")))
            .respond_with(answer(json!({"objects": {"id": "5"}})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);
    c.reset_invoice_to_open(InvoiceId::new(5)).await.unwrap();
    c.reset_credit_note_to_open(CreditNoteId::new(6))
        .await
        .unwrap();
}

fn error_body(status: u16, code: Option<i64>, message: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(json!({
        "error": {"message": message, "code": code, "data": [], "responseCode": status},
        "objects": null
    }))
}

fn cancellation_invoice() -> serde_json::Value {
    json!({"objects": {
        "id": "135169578", "objectName": "Invoice", "invoiceType": "SR",
        "status": "200", "currency": "EUR", "sumGross": "-10",
        "origin": {"id": "135169215", "objectName": "Invoice"}
    }})
}

#[tokio::test]
async fn cancelling_an_invoice_posts_bodyless_and_returns_the_cancellation_invoice() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Invoice/135169215/cancelInvoice"))
        .respond_with(ResponseTemplate::new(201).set_body_json(cancellation_invoice()))
        .expect(1)
        .mount(&server)
        .await;
    let cancelled = client(&server)
        .cancel_invoice(InvoiceId::new(135169215))
        .await
        .unwrap();
    assert_eq!(cancelled.invoice_type, Some(InvoiceType::Cancellation));
    assert!(cancelled.cancels(InvoiceId::new(135169215)));
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].body.is_empty());
}

#[tokio::test]
async fn cancelling_refusals_are_known_codes() {
    for (status, code, message, known) in [
        (
            422,
            421,
            "invoice already canceled",
            KnownCode::AlreadyCancelled,
        ),
        (422, 170, "CreditNote exists", KnownCode::CreditNoteExists),
        (
            200,
            390,
            "Partially Paid Invoices can not be deleted at the moment, book a credit invoice manually",
            KnownCode::PartlyPaid,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Invoice/5/cancelInvoice"))
            .respond_with(error_body(status, Some(code), message))
            .mount(&server)
            .await;
        let err = client(&server)
            .cancel_invoice(InvoiceId::new(5))
            .await
            .unwrap_err();
        assert!(
            matches!(
                &err,
                SevdeskError::Api { status: s, code: Some(c), .. } if *s == status && *c == code
            ),
            "{err:?}"
        );
        assert_eq!(err.known_code(), Some(known));
    }
}

#[tokio::test]
async fn deleting_a_draft_is_a_bodyless_delete_and_an_enshrined_one_is_159() {
    let server = MockServer::start().await;
    for document in ["Invoice/5", "CreditNote/6"] {
        Mock::given(method("DELETE"))
            .and(path(format!("/{document}")))
            .respond_with(answer(json!({"objects": [null]})))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("DELETE"))
        .and(path("/Invoice/7"))
        .respond_with(error_body(
            409,
            Some(159),
            "Invoice can only be deleted in status 100 but is status 200",
        ))
        .mount(&server)
        .await;
    let c = client(&server);
    c.delete_invoice(InvoiceId::new(5)).await.unwrap();
    c.delete_credit_note(CreditNoteId::new(6)).await.unwrap();
    let err = c.delete_invoice(InvoiceId::new(7)).await.unwrap_err();
    assert_eq!(err.known_code(), Some(KnownCode::NotDeletable));
    assert!(matches!(err, SevdeskError::Api { status: 409, .. }));
}

fn pdf_answer(filename: &str, base64_encoded: bool, content: &str) -> ResponseTemplate {
    answer(json!({"objects": {
        "filename": filename, "mimetype": "application/pdf",
        "base64Encoded": base64_encoded, "content": content
    }}))
}

/// "%PDF-1.5\n" in base64.
const PDF_B64: &str = "JVBERi0xLjUK";

#[tokio::test]
async fn the_pdf_is_always_asked_with_prevent_send_by_and_decoded() {
    let server = MockServer::start().await;
    for document in ["Invoice/5", "CreditNote/6"] {
        Mock::given(method("GET"))
            .and(path(format!("/{document}/getPdf")))
            .and(query_param("preventSendBy", "true"))
            .respond_with(pdf_answer("RE-1001.pdf", true, PDF_B64))
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);
    let pdf = c.invoice_pdf(InvoiceId::new(5)).await.unwrap();
    assert_eq!(pdf.bytes, b"%PDF-1.5\n");
    assert_eq!(pdf.filename, "RE-1001.pdf");
    assert_eq!(pdf.mime_type, "application/pdf");
    c.credit_note_pdf(CreditNoteId::new(6)).await.unwrap();
    for request in server.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), Some("preventSendBy=true"));
    }
}

#[tokio::test]
async fn a_draft_pdf_with_the_bare_filename_is_accepted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice/5/getPdf"))
        .respond_with(pdf_answer(".pdf", true, PDF_B64))
        .mount(&server)
        .await;
    let pdf = client(&server)
        .invoice_pdf(InvoiceId::new(5))
        .await
        .unwrap();
    assert_eq!(pdf.filename, ".pdf");
    assert_eq!(pdf.bytes, b"%PDF-1.5\n");
}

#[tokio::test]
async fn a_pdf_that_is_not_base64_or_empty_or_garbled_is_drift() {
    for template in [
        pdf_answer("RE-1.pdf", false, PDF_B64),
        pdf_answer("RE-1.pdf", true, ""),
        pdf_answer("RE-1.pdf", true, "not base64!"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/Invoice/5/getPdf"))
            .respond_with(template)
            .mount(&server)
            .await;
        let err = client(&server)
            .invoice_pdf(InvoiceId::new(5))
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                SevdeskError::DriftGuard {
                    context: "getPdf",
                    ..
                }
            ),
            "{err:?}"
        );
    }
}

#[tokio::test]
async fn the_xml_is_the_objects_string() {
    let server = MockServer::start().await;
    for document in ["Invoice/5", "CreditNote/6"] {
        Mock::given(method("GET"))
            .and(path(format!("/{document}/getXml")))
            .respond_with(answer(json!({"objects": "<?xml version=\"1.0\"?>\n<a/>"})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);
    assert_eq!(
        c.invoice_xml(InvoiceId::new(5)).await.unwrap(),
        "<?xml version=\"1.0\"?>\n<a/>"
    );
    c.credit_note_xml(CreditNoteId::new(6)).await.unwrap();
}

#[tokio::test]
async fn xml_errors_without_a_code_stay_api_errors() {
    for (status, message) in [
        (500, "Missing iban and bic"),
        (400, "This invoice is not an electronic invoice"),
        (503, "Exception when requesting invoices service"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/Invoice/5/getXml"))
            .respond_with(error_body(status, None, message))
            .mount(&server)
            .await;
        let err = client(&server)
            .invoice_xml(InvoiceId::new(5))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, SevdeskError::Api { status: s, code: None, message: m } if *s == status && m == message),
            "{err:?}"
        );
    }
}

#[tokio::test]
async fn an_empty_xml_is_drift() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoice/5/getXml"))
        .respond_with(answer(json!({"objects": ""})))
        .mount(&server)
        .await;
    let err = client(&server)
        .invoice_xml(InvoiceId::new(5))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SevdeskError::DriftGuard {
            context: "getXml",
            ..
        }
    ));
}

#[tokio::test]
async fn the_missing_einvoice_fields_are_named_in_the_message() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/Invoice/5/sendBy"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": {
            "message": "Could not generate ZUGFeRD Invoice since there are validations errors",
            "code": 830,
            "data": {"missing information": ["document_addressCity", "sevClient_bankIban"]},
            "responseCode": 400
        }, "objects": null})))
        .mount(&server)
        .await;
    let err = client(&server)
        .send_invoice_by(InvoiceId::new(5), &SendBy::ENSHRINE)
        .await
        .unwrap_err();
    assert_eq!(err.known_code(), Some(KnownCode::EInvoiceFieldsMissing));
    let message = err.to_string();
    assert!(
        message.contains("document_addressCity, sevClient_bankIban"),
        "{message}"
    );
}

#[tokio::test]
async fn send_via_email_posts_the_typed_body_and_ignores_the_answer() {
    let server = MockServer::start().await;
    for document in ["Invoice/5", "CreditNote/6"] {
        Mock::given(method("POST"))
            .and(path(format!("/{document}/sendViaEmail")))
            .and(body_json(json!({
                "toEmail": "kunde@example.invalid", "subject": "Rechnung",
                "text": "<p>Hallo</p>", "copy": true,
                "additionalAttachments": "12,13", "sendXml": true
            })))
            .respond_with(answer(json!({"objects": {"anything": 1}})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let mail = SendViaEmail {
        to_email: "kunde@example.invalid".into(),
        subject: "Rechnung".into(),
        text: "<p>Hallo</p>".into(),
        copy: true,
        additional_attachments: Some("12,13".into()),
        send_xml: true,
    };
    let c = client(&server);
    c.send_invoice_via_email(InvoiceId::new(5), &mail)
        .await
        .unwrap();
    c.send_credit_note_via_email(CreditNoteId::new(6), &mail)
        .await
        .unwrap();
}

#[tokio::test]
async fn send_via_email_leaves_absent_optionals_out() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Invoice/5/sendViaEmail"))
        .and(body_json(json!({
            "toEmail": "a@example.invalid", "subject": "S", "text": "T", "copy": false
        })))
        .respond_with(answer(json!({"objects": null})))
        .expect(1)
        .mount(&server)
        .await;
    let mail = SendViaEmail {
        to_email: "a@example.invalid".into(),
        subject: "S".into(),
        text: "T".into(),
        copy: false,
        additional_attachments: None,
        send_xml: false,
    };
    client(&server)
        .send_invoice_via_email(InvoiceId::new(5), &mail)
        .await
        .unwrap();
}
