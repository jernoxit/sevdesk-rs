use crate::client::SevdeskClient;
use crate::document_payload::{DocumentParameter, SaveCreditNote, SendBy, SendViaEmail};
use crate::error::SevdeskError;
use crate::ids::CreditNoteId;
use crate::payload::BookDocumentAmount;
use crate::query::CreditNoteQuery;
use crate::wire::{CreditNote, DocumentPdf, SavedCreditNote, xml_from_body};
use reqwest::Method;

impl SevdeskClient {
    /// `GET /CreditNote`.
    pub async fn list_credit_notes(
        &self,
        query: &CreditNoteQuery,
    ) -> Result<Vec<CreditNote>, SevdeskError> {
        self.get("/CreditNote", &query.pairs()).await?.list()
    }

    /// `GET /CreditNote/{id}`.
    pub async fn credit_note(&self, id: CreditNoteId) -> Result<CreditNote, SevdeskError> {
        self.get(&format!("/CreditNote/{id}"), &[]).await?.one()
    }

    /// `PUT /CreditNote/{id}/bookAmount`; the amount carries the sign of the
    /// transaction (measured: −58.31 pays a credit note of 58.31).
    pub async fn book_credit_note_amount(
        &self,
        id: CreditNoteId,
        booking: &BookDocumentAmount,
    ) -> Result<(), SevdeskError> {
        self.send_json(
            Method::PUT,
            &format!("/CreditNote/{id}/bookAmount"),
            booking,
        )
        .await
        .map(drop)
    }
}

impl SevdeskClient {
    /// `POST /CreditNote/Factory/saveCreditNote`: creates a DRAFT. Give it a
    /// number from [`SevdeskClient::next_credit_note_number`], or `sendBy`
    /// fails.
    pub async fn save_credit_note(
        &self,
        save: &SaveCreditNote,
    ) -> Result<SavedCreditNote, SevdeskError> {
        self.send_json(Method::POST, "/CreditNote/Factory/saveCreditNote", save)
            .await?
            .one()
    }

    /// `PUT /CreditNote/{id}/sendBy`: enshrines a draft (status 200).
    pub async fn send_credit_note_by(
        &self,
        id: CreditNoteId,
        send: &SendBy,
    ) -> Result<CreditNote, SevdeskError> {
        self.send_json(Method::PUT, &format!("/CreditNote/{id}/sendBy"), send)
            .await?
            .one()
    }
}

impl SevdeskClient {
    /// `PUT /CreditNote/{id}/changeParameter`, e.g. the document language.
    pub async fn change_credit_note_parameter(
        &self,
        id: CreditNoteId,
        parameter: DocumentParameter,
    ) -> Result<(), SevdeskError> {
        self.send_json(
            Method::PUT,
            &format!("/CreditNote/{id}/changeParameter"),
            &parameter,
        )
        .await
        .map(drop)
    }
}

impl SevdeskClient {
    /// `PUT /CreditNote/{id}/resetToOpen`: takes back all bookings (the
    /// transactions return to status 100).
    pub async fn reset_credit_note_to_open(&self, id: CreditNoteId) -> Result<(), SevdeskError> {
        self.send_empty(Method::PUT, &format!("/CreditNote/{id}/resetToOpen"))
            .await
            .map(drop)
    }
}

impl SevdeskClient {
    /// `DELETE /CreditNote/{id}` (undocumented; measured: a draft deletes with 200,
    /// an enshrined credit note answers 409 with code 159, [`crate::KnownCode::NotDeletable`]).
    pub async fn delete_credit_note(&self, id: CreditNoteId) -> Result<(), SevdeskError> {
        self.send_empty(Method::DELETE, &format!("/CreditNote/{id}"))
            .await
            .map(drop)
    }

    /// `GET /CreditNote/{id}/getPdf`, always with `preventSendBy=true`: without it,
    /// rendering a draft enshrines it. There is no way to switch it off.
    ///
    /// A draft renders too (filename `".pdf"`, see [`DocumentPdf`]).
    pub async fn credit_note_pdf(&self, id: CreditNoteId) -> Result<DocumentPdf, SevdeskError> {
        let query = [("preventSendBy", "true".to_owned())];
        let body = self
            .get(&format!("/CreditNote/{id}/getPdf"), &query)
            .await?;
        DocumentPdf::from_body(&body)
    }

    /// `GET /CreditNote/{id}/getXml`: the e-invoice XML, only after enshrining and
    /// only for an e-invoice. Measured: the failures (missing bank data, a
    /// non-electronic document) come as 500, 503 or 400, mostly without a
    /// code, so match on the status and message.
    pub async fn credit_note_xml(&self, id: CreditNoteId) -> Result<String, SevdeskError> {
        let body = self.get(&format!("/CreditNote/{id}/getXml"), &[]).await?;
        xml_from_body(&body)
    }

    /// `POST /CreditNote/{id}/sendViaEmail`. Per spec, unmeasured: the answer's
    /// `objects` is not decoded.
    pub async fn send_credit_note_via_email(
        &self,
        id: CreditNoteId,
        mail: &SendViaEmail,
    ) -> Result<(), SevdeskError> {
        self.send_json(
            Method::POST,
            &format!("/CreditNote/{id}/sendViaEmail"),
            mail,
        )
        .await
        .map(drop)
    }
}
