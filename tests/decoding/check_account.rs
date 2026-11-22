//! Fixture decoding of check accounts.

use super::{eur, serving};
use sevdesk::*;

#[tokio::test]
async fn check_account() {
    let (_s, c) = serving("/CheckAccount/6306017", "check_account").await;
    let a = c.check_account(CheckAccountId::new(6306017)).await.unwrap();
    assert_eq!(a.id, CheckAccountId::new(6306017));
    assert_eq!(a.name.as_deref(), Some("Probe Stripe Import PROBE-R1"));
    assert_eq!(a.kind.as_deref(), Some("online"));
    assert_eq!(a.accounting_number, Some(1201));
    assert_eq!(a.auto_map_transaction, Some(true));
    assert_eq!(a.balance, Some(eur(0)));
    assert_eq!(a.currency, CurrencyCode::EUR);
}

#[tokio::test]
async fn check_account_list() {
    let (_s, c) = serving("/CheckAccount", "check_account_list").await;
    let list = c.list_check_accounts().await.unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().any(|a| a.kind.as_deref() == Some("offline")));
    assert!(list.iter().all(|a| a.currency == CurrencyCode::EUR));
}

#[tokio::test]
async fn balance_at_date_reads_a_bare_negative_string() {
    let (_s, c) = serving("/CheckAccount/6306017/getBalanceAtDate", "balance_at_date").await;
    let date = time::macros::date!(2026 - 09 - 29);
    assert_eq!(
        c.balance_at_date(CheckAccountId::new(6306017), date)
            .await
            .unwrap(),
        eur(-139118)
    );
}
