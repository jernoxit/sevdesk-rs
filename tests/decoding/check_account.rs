//! Fixture decoding of check accounts.

use super::{eur, sample_positions, serving};
use sevdesk::*;

#[tokio::test]
async fn check_account() {
    let (_s, c) = serving("/CheckAccount/6306017", "check_account").await;
    let a = c.check_account(CheckAccountId::new(6306017)).await.unwrap();
    assert_eq!(a.id, CheckAccountId::new(6306017));
    assert_eq!(a.name.as_deref(), Some("Probe Stripe Import PROBE-R1"));
    assert_eq!(a.kind, Some(CheckAccountType::Online));
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
    assert!(
        list.iter()
            .any(|a| a.kind == Some(CheckAccountType::Offline))
    );
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

#[test]
fn sample_check_account_list_reads_every_measured_form() {
    let list = sample_positions::<CheckAccount>("sample_check_account_list");
    assert_eq!(list.len(), 2);
    let online = &list[0];
    assert_eq!(online.kind, Some(CheckAccountType::Online));
    assert_eq!(online.import_type, Some(CheckAccountImportType::Csv));
    assert_eq!(online.status, Some(CheckAccountStatus::Active));
    assert_eq!(online.default_account, Some(true));
    assert_eq!(online.auto_sync_transactions, Some(false));
    assert_eq!(online.base_account, Some(0));
    assert_eq!(online.priority, Some(0));
    assert_eq!(online.balance, Some(eur(0)));
    assert_eq!(
        online.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert!(online.create.is_some() && online.update.is_some());
    // Never filled in any measurement.
    assert_eq!(
        (online.iban.as_deref(), online.bic.as_deref()),
        (None, None)
    );
    assert_eq!(online.bank_server, None);
    assert_eq!(online.last_sync, None);

    // An offline account: no import type, `null` balance, base account.
    let offline = &list[1];
    assert_eq!(offline.kind, Some(CheckAccountType::Offline));
    assert_eq!(offline.import_type, None);
    assert_eq!(offline.balance, None);
    assert_eq!(offline.base_account, Some(1));
    assert_eq!(offline.auto_sync_transactions, Some(true));
}

#[test]
fn check_account_spec_only_enum_values_read() {
    // Synthetic: `register`, `MT940` and status `0` are in the OpenAPI spec only.
    let a: CheckAccount = serde_json::from_str(
        r#"{"id":"1","currency":"EUR","type":"register","importType":"MT940","status":"0"}"#,
    )
    .unwrap();
    assert_eq!(a.kind, Some(CheckAccountType::Register));
    assert_eq!(a.import_type, Some(CheckAccountImportType::Mt940));
    assert_eq!(a.status, Some(CheckAccountStatus::Inactive));
}
