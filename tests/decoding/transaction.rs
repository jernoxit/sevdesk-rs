//! Fixture decoding of transactions.

use super::{client, eur, fixture, serving};
use sevdesk::*;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn transaction_by_id() {
    let (_s, c) = serving("/CheckAccountTransaction/1897813785", "transaction").await;
    let t = c.transaction(TransactionId::new(1897813785)).await.unwrap();
    assert_eq!(t.amount, Some(eur(-1500)));
    assert_eq!(t.status, Some(TransactionStatus::Booked));
    assert_eq!(t.check_account.unwrap().id, CheckAccountId::new(6306017));
    assert_eq!(t.paymt_purpose.as_deref(), Some("probe diff B tx3"));
    assert!(!t.is_deleted());
    // The offset is kept: 12:00 Berlin, not UTC.
    assert_eq!(t.value_date.unwrap().offset().whole_hours(), 2);
}

#[tokio::test]
async fn deleted_transaction_by_id_is_marked_deleted() {
    let (_s, c) = serving("/CheckAccountTransaction/1897811012", "transaction_deleted").await;
    let t = c.transaction(TransactionId::new(1897811012)).await.unwrap();
    assert!(t.is_deleted());
    assert_eq!(t.status, Some(TransactionStatus::Created));
}

#[tokio::test]
async fn transaction_list() {
    let (_s, c) = serving("/CheckAccountTransaction", "transaction_list").await;
    let list = c
        .list_transactions(&TransactionQuery::new(CheckAccountId::new(6306017)))
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].amount, Some(eur(3451)));
    assert_eq!(list[0].status, Some(TransactionStatus::Linked));
}

#[tokio::test]
async fn transaction_log_reads_number_amount_and_document() {
    let (_s, c) = serving("/CheckAccountTransactionLog", "transaction_log").await;
    let log = c
        .transaction_log(TransactionId::new(1897813785))
        .await
        .unwrap();
    assert_eq!(log.len(), 1);
    // `amountPaid` is a JSON number (-15) here.
    assert_eq!(log[0].amount_paid, Some(eur(-1500)));
    assert_eq!(
        log[0].object,
        Some(BookedDocument::Voucher(VoucherId::new(156247137)))
    );
    assert_eq!(log[0].from_status, Some(TransactionStatus::Created));
    assert_eq!(log[0].to_status, Some(TransactionStatus::Booked));
}

#[tokio::test]
async fn error_422_offline_account() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(422).set_body_string(fixture("error_422_offline_account")),
        )
        .mount(&server)
        .await;
    let err = client(&server)
        .create_transaction(&sample_transaction())
        .await
        .unwrap_err();
    assert!(err.is_offline_account(), "{err:?}");
    assert!(matches!(
        err,
        SevdeskError::Api {
            status: 422,
            code: None,
            ..
        }
    ));
}

fn sample_transaction() -> NewTransaction {
    NewTransaction {
        value_date: time::macros::datetime!(2026-08-15 12:00 +2),
        entry_date: None,
        amount: eur(-100),
        payee_payer_name: "Stripe Payments Europe, Limited".into(),
        paymt_purpose: Some("x".into()),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(CheckAccountId::new(6306016)),
        source_transaction: None,
        target_transaction: None,
    }
}
