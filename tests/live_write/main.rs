//! WRITE calls against the sevDesk TEST account. Ignored by default; run only
//! through `just test-live --test live_write` (the recipe sources
//! the credentials; the harness reads `SEVDESK_API_KEY`, the library never
//! does). Skips with a message when it is unset.
//!
//! Every object carries the marker `SS-LIVE-<unix ts>-<test>` in
//! `description` / `paymtPurpose`. The tests share one account, so they run one
//! at a time; each sweeps ALL leftovers with the `SS-LIVE-` prefix before it
//! starts and after it ends (also when its body panicked).

#[path = "../common/scrub.rs"]
mod scrub;

mod captures;
mod contacts;
mod credit_notes;
mod docs;
mod harness;
mod invoices;
mod transactions;
mod vouchers;
