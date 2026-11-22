//! The `sendBy` payload.

use crate::status::*;
use serde::{Serialize, Serializer};

/// `PUT /Invoice/{id}/sendBy` and `PUT /CreditNote/{id}/sendBy`.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendBy {
    pub send_type: SendType,
    pub send_draft: bool,
}

impl SendBy {
    /// Finalize without sending anything: the document gets status 200 and its
    /// number. It is enshrined only if the account setting "Automatische
    /// Festschreibung von Dokumenten bei Fertigstellung" is on (measured
    /// 2026-11-05; with the setting off, `enshrined` stays `None`). For a lock
    /// independent of the setting, call `enshrine_invoice` or
    /// `enshrine_credit_note` afterwards; on an already enshrined document that
    /// is an idempotent no-op.
    ///
    /// sevDesk can answer a write with an error body such as
    /// `cURL error 28 … 10001 milliseconds` (its internal accounting-service
    /// timeout). The write may have happened: re-read the document before
    /// retrying.
    pub const FINALIZE: Self = Self {
        send_type: SendType::Vpdf,
        send_draft: false,
    };
}

/// `POST /Invoice/{id}/sendViaEmail` and `POST /CreditNote/{id}/sendViaEmail`.
///
/// Per spec, unmeasured.
#[derive(Debug, Clone)]
pub struct SendViaEmail {
    pub to_email: String,
    pub subject: String,
    /// The mail body, HTML.
    pub text: String,
    /// Send a copy to the sender.
    pub copy: bool,
    pub options: SendViaEmailOptions,
}

/// The optional fields of a [`SendViaEmail`]; unmeasured like the call.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SendViaEmailOptions {
    /// Further documents to attach, as the spec's comma-separated list of ids;
    /// left out when `None`.
    pub additional_attachments: Option<String>,
    /// Attach the e-invoice XML; sent only when `true`.
    pub send_xml: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SendViaEmailWire {
    to_email: String,
    subject: String,
    text: String,
    copy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_attachments: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    send_xml: bool,
}

impl From<SendViaEmail> for SendViaEmailWire {
    fn from(mail: SendViaEmail) -> Self {
        let SendViaEmail {
            to_email,
            subject,
            text,
            copy,
            options,
        } = mail;
        let SendViaEmailOptions {
            additional_attachments,
            send_xml,
        } = options;
        Self {
            to_email,
            subject,
            text,
            copy,
            additional_attachments,
            send_xml,
        }
    }
}

impl Serialize for SendViaEmail {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SendViaEmailWire::from(self.clone()).serialize(s)
    }
}
