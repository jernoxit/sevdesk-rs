use crate::codec::deserialize_i64_opt;
use crate::ids::*;
use serde::Deserialize;
use time::OffsetDateTime;

/// A stored file (`Document`), as `saveVoucher` embeds it when a file is
/// attached. Only that call returns the full object; the voucher itself
/// carries a reference.
///
/// Every key a recording shows filled is a field. `additionalInformation`,
/// `description`, `documentNumber` and `mimeType` are `null` in both recordings
/// and not in the OpenAPI spec (it has no schema for a document), so they are
/// not read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: DocumentId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub create: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub update: Option<OffsetDateTime>,
    /// The sevDesk user who created the document.
    #[serde(default)]
    pub create_user: Option<ObjectRef<SevUserId>>,
    /// The sevDesk user who last changed the document.
    #[serde(default)]
    pub update_user: Option<ObjectRef<SevUserId>>,
    #[serde(default)]
    pub sev_client: Option<ObjectRef<SevClientId>>,
    /// The stored file's name at sevDesk: a hash, not the uploaded name.
    #[serde(default)]
    pub filename: Option<String>,
    /// Measured as `"50"` only; no value set is known, so it stays a number.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub status: Option<i64>,
    /// Measured as `"pdf"` only.
    #[serde(default)]
    pub extension: Option<String>,
    /// Size in bytes; the wire has a string.
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub filesize: Option<i64>,
    /// Hex digest of the content.
    #[serde(default)]
    pub content_hash: Option<String>,
}
