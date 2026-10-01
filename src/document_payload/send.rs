//! The `sendBy` payload.

use crate::status::*;
use serde::Serialize;

/// `PUT /Invoice/{id}/sendBy` and `PUT /CreditNote/{id}/sendBy`.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendBy {
    pub send_type: SendType,
    pub send_draft: bool,
}

impl SendBy {
    /// Enshrine without sending anything.
    pub const ENSHRINE: Self = Self {
        send_type: SendType::Vpdf,
        send_draft: false,
    };
}

/// `POST /Invoice/{id}/sendViaEmail` and `POST /CreditNote/{id}/sendViaEmail`.
///
/// Per spec, unmeasured.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendViaEmail {
    pub to_email: String,
    pub subject: String,
    /// The mail body, HTML.
    pub text: String,
    /// Send a copy to the sender.
    pub copy: bool,
    /// Further documents to attach, as the spec's comma-separated list of ids;
    /// left out when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_attachments: Option<String>,
    /// Attach the e-invoice XML; sent only when `true`.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub send_xml: bool,
}
