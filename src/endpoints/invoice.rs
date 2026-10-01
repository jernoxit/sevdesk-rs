use crate::client::SevdeskClient;
use crate::document_payload::{DocumentParameter, SaveInvoice, SendBy, SendViaEmail};
use crate::error::SevdeskError;
use crate::ids::InvoiceId;
use crate::payload::BookDocumentAmount;
use crate::query::InvoiceQuery;
use crate::wire::{DocumentPdf, Invoice, SavedInvoice, xml_from_body};
use reqwest::Method;

impl SevdeskClient {
    /// `GET /Invoice`. `customer_internal_note` matches EXACTLY and returns the
    /// original and its cancellation invoice alike.
    ///
    /// With `update_after` set, every invoice must carry an `update` at or
    /// after it; otherwise sevDesk ignored the filter and the answer is
    /// [`SevdeskError::DriftGuard`].
    pub async fn list_invoices(&self, query: &InvoiceQuery) -> Result<Vec<Invoice>, SevdeskError> {
        let invoices: Vec<Invoice> = self.get("/Invoice", &query.pairs()).await?.list()?;
        if let Some(after) = query.update_after {
            for invoice in &invoices {
                if !invoice.update.is_some_and(|update| update >= after) {
                    return Err(SevdeskError::DriftGuard {
                        context: "GET /Invoice updateAfter",
                        detail: format!(
                            "invoice {} has update {:?}, not at or after {after}",
                            invoice.id, invoice.update
                        ),
                    });
                }
            }
        }
        Ok(invoices)
    }

    /// `GET /Invoice/{id}`.
    pub async fn invoice(&self, id: InvoiceId) -> Result<Invoice, SevdeskError> {
        self.get(&format!("/Invoice/{id}"), &[]).await?.one()
    }

    /// `PUT /Invoice/{id}/bookAmount`.
    pub async fn book_invoice_amount(
        &self,
        id: InvoiceId,
        booking: &BookDocumentAmount,
    ) -> Result<(), SevdeskError> {
        self.send_json(Method::PUT, &format!("/Invoice/{id}/bookAmount"), booking)
            .await
            .map(drop)
    }
}

impl SevdeskClient {
    /// `POST /Invoice/Factory/saveInvoice`: creates a DRAFT without a number.
    pub async fn save_invoice(&self, save: &SaveInvoice) -> Result<SavedInvoice, SevdeskError> {
        self.send_json(Method::POST, "/Invoice/Factory/saveInvoice", save)
            .await?
            .one()
    }

    /// `PUT /Invoice/{id}/sendBy`: enshrines a draft (status 200) and assigns
    /// its number; answers with the invoice.
    pub async fn send_invoice_by(
        &self,
        id: InvoiceId,
        send: &SendBy,
    ) -> Result<Invoice, SevdeskError> {
        self.send_json(Method::PUT, &format!("/Invoice/{id}/sendBy"), send)
            .await?
            .one()
    }
}

impl SevdeskClient {
    /// `PUT /Invoice/{id}/changeParameter`, e.g. the document language.
    /// Measured 2026-10-01: `en_US` alone, without a render call, makes the
    /// next PDF English, before `sendBy` and after it. On an e-invoice DRAFT it
    /// answers 503 like [`Self::invoice_pdf`]: change it after enshrining.
    pub async fn change_invoice_parameter(
        &self,
        id: InvoiceId,
        parameter: DocumentParameter,
    ) -> Result<(), SevdeskError> {
        self.send_json(
            Method::PUT,
            &format!("/Invoice/{id}/changeParameter"),
            &parameter,
        )
        .await
        .map(drop)
    }
}

impl SevdeskClient {
    /// `PUT /Invoice/{id}/resetToOpen`: takes back all bookings (the
    /// transactions return to status 100). Refused with code 170 when a
    /// credit note stands beside the invoice.
    pub async fn reset_invoice_to_open(&self, id: InvoiceId) -> Result<(), SevdeskError> {
        self.send_empty(Method::PUT, &format!("/Invoice/{id}/resetToOpen"))
            .await
            .map(drop)
    }
}

impl SevdeskClient {
    /// `DELETE /Invoice/{id}` (undocumented; measured: a draft deletes with 200,
    /// an enshrined invoice answers 409 with code 159, [`crate::KnownCode::NotDeletable`]).
    pub async fn delete_invoice(&self, id: InvoiceId) -> Result<(), SevdeskError> {
        self.send_empty(Method::DELETE, &format!("/Invoice/{id}"))
            .await
            .map(drop)
    }

    /// `GET /Invoice/{id}/getPdf`, always with `preventSendBy=true`: without it,
    /// rendering a draft enshrines it. There is no way to switch it off.
    ///
    /// A draft renders too (filename `".pdf"`, see [`DocumentPdf`]), except an
    /// e-invoice draft: it has no number yet, and sevDesk answers 503 ("Invoice
    /// number is required", measured 2026-10-01).
    pub async fn invoice_pdf(&self, id: InvoiceId) -> Result<DocumentPdf, SevdeskError> {
        let query = [("preventSendBy", "true".to_owned())];
        let body = self.get(&format!("/Invoice/{id}/getPdf"), &query).await?;
        DocumentPdf::from_body(&body)
    }

    /// `GET /Invoice/{id}/getXml`: the e-invoice XML, only after enshrining and
    /// only for an e-invoice. Measured: the failures (missing bank data, a
    /// non-electronic document) come as 500, 503 or 400, mostly without a
    /// code, so match on the status and message.
    pub async fn invoice_xml(&self, id: InvoiceId) -> Result<String, SevdeskError> {
        let body = self.get(&format!("/Invoice/{id}/getXml"), &[]).await?;
        xml_from_body(&body)
    }

    /// `POST /Invoice/{id}/sendViaEmail`. Per spec, unmeasured: the answer's
    /// `objects` is not decoded.
    pub async fn send_invoice_via_email(
        &self,
        id: InvoiceId,
        mail: &SendViaEmail,
    ) -> Result<(), SevdeskError> {
        self.send_json(Method::POST, &format!("/Invoice/{id}/sendViaEmail"), mail)
            .await
            .map(drop)
    }
}

impl SevdeskClient {
    /// `POST /Invoice/{id}/cancelInvoice`: answers 201 with the cancellation
    /// invoice (type `SR`, see [`Invoice::cancels`]). Refused with
    /// [`crate::KnownCode::AlreadyCancelled`] (422, 421), [`crate::KnownCode::CreditNoteExists`]
    /// (422, 170), or, as an error body inside HTTP 200,
    /// [`crate::KnownCode::PartlyPaid`] (390).
    pub async fn cancel_invoice(&self, id: InvoiceId) -> Result<Invoice, SevdeskError> {
        self.send_empty(Method::POST, &format!("/Invoice/{id}/cancelInvoice"))
            .await?
            .one()
    }
}
