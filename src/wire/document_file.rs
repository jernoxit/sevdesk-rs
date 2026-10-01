use crate::client::Body;
use crate::error::SevdeskError;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;

/// A rendered PDF of an invoice or credit note, decoded.
///
/// `filename` is `".pdf"` (no stem) on a draft, which has no number yet; it is
/// returned as sevDesk sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPdf {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub mime_type: String,
}

/// `objects` of `getPdf`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfWire {
    #[serde(default)]
    filename: Option<String>,
    #[serde(default)]
    mimetype: Option<String>,
    #[serde(default)]
    base64_encoded: Option<bool>,
    #[serde(default)]
    content: Option<String>,
}

const PDF_CONTEXT: &str = "getPdf";

impl DocumentPdf {
    /// Reads a `getPdf` answer. The load-bearing fields are `base64Encoded`
    /// and `content`, not the filename. Content that is not flagged as base64
    /// or is empty, or that does not decode, is drift: sevDesk changed shape.
    pub(crate) fn from_body(body: &Body) -> Result<Self, SevdeskError> {
        let PdfWire {
            filename,
            mimetype,
            base64_encoded,
            content,
        } = body.plain(PDF_CONTEXT)?;
        let drift = |detail: String| SevdeskError::DriftGuard {
            context: PDF_CONTEXT,
            detail,
        };
        if base64_encoded != Some(true) {
            return Err(drift(format!("`base64Encoded` is {base64_encoded:?}")));
        }
        let content = content
            .filter(|content| !content.is_empty())
            .ok_or_else(|| drift("`content` is missing or empty".into()))?;
        let bytes = STANDARD
            .decode(content)
            .map_err(|e| drift(format!("`content` is not base64: {e}")))?;
        Ok(Self {
            bytes,
            filename: filename.unwrap_or_default(),
            mime_type: mimetype.unwrap_or_default(),
        })
    }
}

/// Reads a `getXml` answer: `objects` is the XML text itself.
pub(crate) fn xml_from_body(body: &Body) -> Result<String, SevdeskError> {
    let xml: String = body.plain("getXml")?;
    if xml.trim().is_empty() {
        return Err(SevdeskError::DriftGuard {
            context: "getXml",
            detail: "`objects` is an empty string".into(),
        });
    }
    Ok(xml)
}
