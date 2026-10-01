//! What we send to create invoices, credit notes and their customers.
//!
//! The drafts are plain data: every business value is the caller's, the wire
//! form (`objectName`, `mapAll`, `status`, `discount`, ...) is private. The facts measured on
//! the test account (2026-09-29) are documented where they apply: the draft and
//! `sendBy` flow in [`invoice`], the credit note's number and booking category
//! in [`credit_note`], the amount format in [`position`].

mod contact;
mod credit_note;
mod invoice;
mod parameter;
mod position;
mod send;

pub use contact::*;
pub use credit_note::*;
pub use invoice::*;
pub use parameter::*;
pub use position::*;
pub use send::*;
