use crate::codec::deserialize_i64_opt;
use crate::wire::Guard;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Deserializer, de};

/// The answer of `changeParameter` on an invoice or a credit note: the
/// document's layout parameters after the change.
///
/// The two answers differ in shape: a credit note's carries `result`, an
/// invoice's does not.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DocumentParameters {
    /// `1` on a credit note (measured 2026-11-22); absent on an invoice.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub result: Option<i64>,
    #[serde(default)]
    pub meta_data: Option<DocumentMetaData>,
}

/// `metaData` of a [`DocumentParameters`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DocumentMetaData {
    /// sevDesk's id of the rendered document (a hash).
    #[serde(default)]
    pub doc_id: Option<String>,
    /// Every parameter the document knows, with its choices and current value.
    #[serde(default)]
    pub parameters: Vec<DocumentParameterInfo>,
    /// The re-rendered PDF, decoded from base64.
    #[serde(default, deserialize_with = "pdf_opt")]
    pub pdf: Option<Vec<u8>>,
}

/// One layout parameter of a document (`language`, `template`, ...).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DocumentParameterInfo {
    pub key: String,
    /// The display name, in the account's language.
    #[serde(default)]
    pub name: Option<String>,
    /// The choices.
    #[serde(default)]
    pub values: Vec<DocumentParameterChoice>,
    /// The current value; one of the choices' `value` (empty when none is set).
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub visible: Option<bool>,
}

/// One choice of a [`DocumentParameterInfo`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DocumentParameterChoice {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub translation_code: Option<String>,
    /// `null` on the built-in choices of most parameters, a flag on layouts.
    #[serde(default)]
    pub is_custom: Option<bool>,
}

impl Guard for DocumentParameters {
    const NAME: &'static str = "changeParameter";
    fn load_bearing_present(&self) -> bool {
        self.meta_data.is_some()
    }
}

fn pdf_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
    Option::<String>::deserialize(d)?
        .map(|text| {
            STANDARD
                .decode(text)
                .map_err(|e| de::Error::custom(format!("`pdf` is not base64: {e}")))
        })
        .transpose()
}
