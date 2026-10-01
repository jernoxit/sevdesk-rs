//! The `changeParameter` payload.

use crate::status::DocumentLanguage;
use serde::{Serialize, Serializer};

/// A parameter of a document's rendering, for `PUT /Invoice/{id}/changeParameter`
/// and `PUT /CreditNote/{id}/changeParameter`.
///
/// Only the language is typed so far. Measured: sevDesk stores `en` when sent
/// `en_US`, and `changeParameter` alone changes the PDF (2026-10-01); no
/// `render` call has to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentParameter {
    Language(DocumentLanguage),
}

#[derive(Serialize)]
struct DocumentParameterWire {
    key: &'static str,
    value: DocumentLanguage,
}

impl From<DocumentParameter> for DocumentParameterWire {
    fn from(parameter: DocumentParameter) -> Self {
        match parameter {
            DocumentParameter::Language(value) => Self {
                key: "language",
                value,
            },
        }
    }
}

impl Serialize for DocumentParameter {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        DocumentParameterWire::from(*self).serialize(s)
    }
}
