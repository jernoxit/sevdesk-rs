//! The voucher resource group: the voucher, its positions, the `saveVoucher`
//! answer, the attached document and the booking log entry, one file each.

mod attachment;
mod document;
mod log;
mod position;
mod saved;

pub use attachment::Document;
pub use document::Voucher;
pub use log::VoucherLog;
pub use position::VoucherPosition;
pub use saved::SavedVoucher;
