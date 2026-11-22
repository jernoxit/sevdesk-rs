use super::{Document, Voucher, VoucherPosition};
use crate::wire::Guard;
use serde::Deserialize;

/// The answer of `saveVoucher`: the voucher and ALL its positions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SavedVoucher {
    pub voucher: Voucher,
    #[serde(default)]
    pub voucher_pos: Vec<VoucherPosition>,
    /// Measured as `"not relevant"`.
    #[serde(default)]
    pub vatdrop: Option<String>,
    /// The attached file, as a full object; only present when a file was
    /// attached with the save.
    #[serde(default)]
    pub document: Option<Document>,
    /// Form not measured; type from the OpenAPI spec (a request artifact there).
    #[serde(default)]
    pub filename: Option<String>,
}

impl Guard for SavedVoucher {
    const NAME: &'static str = "saveVoucher";
    fn load_bearing_present(&self) -> bool {
        self.voucher.load_bearing_present()
    }
}
