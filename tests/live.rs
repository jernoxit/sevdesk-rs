//! Read-only calls against the sevDesk TEST account. Ignored by default; run
//! with `SEVDESK_API_KEY` in the process environment (the test harness reads
//! it, the library never does). Skips with a message when it is unset.
//! No write calls here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helper code outside #[test] functions: a failure here is a test failure"
)]

use sevdesk::*;

const IMPORT_ACCOUNT: i64 = 6306017;
const TRANSACTION_WITH_LOG: i64 = 1897813785;
const DELETED_TRANSACTION: i64 = 1897811012;
const PAID_VOUCHER: i64 = 156244013;
const RE_1015: i64 = 135171691;

fn live() -> Option<SevdeskClient> {
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping live test");
        return None;
    };
    Some(SevdeskClient::new(SevdeskClientConfig::new(ApiToken::new(token))).expect("client"))
}

#[tokio::test]
#[ignore = "needs SEVDESK_API_KEY"]
async fn live_bookkeeping_system_version() {
    let Some(c) = live() else { return };
    assert_eq!(c.bookkeeping_system_version().await.unwrap(), "2.0");
}

#[tokio::test]
#[ignore = "needs SEVDESK_API_KEY"]
async fn live_check_accounts() {
    let Some(c) = live() else { return };
    let accounts = c.list_check_accounts().await.unwrap();
    let import = accounts
        .iter()
        .find(|a| a.id == CheckAccountId::new(IMPORT_ACCOUNT))
        .expect("import account");
    assert_eq!(import.kind.as_deref(), Some("online"));
    let one = c.check_account(import.id).await.unwrap();
    assert_eq!(one.accounting_number, Some(1201));
    c.balance_at_date(import.id, time::macros::date!(2026 - 09 - 29))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "needs SEVDESK_API_KEY"]
async fn live_transactions() {
    let Some(c) = live() else { return };
    let t = c
        .transaction(TransactionId::new(TRANSACTION_WITH_LOG))
        .await
        .unwrap();
    assert_eq!(t.status, Some(TransactionStatus::Booked));
    assert!(!t.is_deleted());
    let log = c.transaction_log(t.id).await.unwrap();
    assert!(
        log.iter()
            .any(|e| matches!(e.object, Some(BookedDocument::Voucher(_))))
    );

    let gone = c
        .transaction(TransactionId::new(DELETED_TRANSACTION))
        .await
        .unwrap();
    assert!(gone.is_deleted());
    let mut q = TransactionQuery::new(CheckAccountId::new(IMPORT_ACCOUNT));
    q.page = Some(Page::new(1000, 0).unwrap());
    let list = c.list_transactions(&q).await.unwrap();
    assert!(list.iter().all(|t| !t.is_deleted()));
    assert!(!list.iter().any(|t| t.id == gone.id), "list omits deleted");

    let err = c.transaction(TransactionId::new(1)).await.unwrap_err();
    assert_eq!(err.known_code(), Some(KnownCode::NotFound));
}

#[tokio::test]
#[ignore = "needs SEVDESK_API_KEY"]
async fn live_vouchers() {
    let Some(c) = live() else { return };
    let v = c.voucher(VoucherId::new(PAID_VOUCHER)).await.unwrap();
    assert_eq!(v.status, Some(VoucherStatus::Paid));
    assert_eq!(v.paid_amount, v.sum_gross);
    let pos = c.voucher_positions(v.id).await.unwrap();
    assert_eq!(pos.len(), 2);
    let found = c
        .list_vouchers(&VoucherQuery {
            description_like: Some("M11".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!found.is_empty());
}

#[tokio::test]
#[ignore = "needs SEVDESK_API_KEY"]
async fn live_invoices_credit_notes_and_guidance() {
    let Some(c) = live() else { return };
    let list = c
        .list_invoices(&InvoiceQuery {
            customer_internal_note: Some("PROBE-R1-E3-pos-pct".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.len(), 2, "exact note filter: original and storno");
    let sr = list
        .iter()
        .find(|i| i.invoice_type == Some(InvoiceType::Cancellation))
        .unwrap();
    assert_eq!(sr.origin.unwrap().id, InvoiceId::new(RE_1015));
    c.invoice(InvoiceId::new(RE_1015)).await.unwrap();

    let note = c.credit_note(CreditNoteId::new(2996041)).await.unwrap();
    assert_eq!(note.credit_note_number.as_deref(), Some("GU-1006"));
    let notes = c
        .list_credit_notes(&CreditNoteQuery {
            customer_internal_note: Some("PROBE-R1-CN-REF-002".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(notes.len(), 1);

    let guidance = c.receipt_guidance_for_expense().await.unwrap();
    let fees = guidance
        .iter()
        .find(|g| g.account_datev_id == AccountDatevId::new(4285))
        .unwrap();
    assert_eq!(fees.account_number.as_deref(), Some("4970"));
}
