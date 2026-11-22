//! Fixture decoding of transactions.

use super::{client, eur, fixture, sample_positions, serving};
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
    let mut new = NewTransaction {
        value_date: time::macros::datetime!(2026-08-15 12:00 +2),
        amount: eur(-100),
        payee_payer_name: "Stripe Payments Europe, Limited".into(),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(CheckAccountId::new(6306016)),
        options: Default::default(),
    };
    new.options.paymt_purpose = Some("x".into());
    new
}

#[test]
fn sample_transaction_list_reads_every_measured_form() {
    let list = sample_positions::<Transaction>("sample_transaction_list");
    assert_eq!(list.len(), 2);
    let by_id = |id: i64| {
        list.iter()
            .find(|t| t.id == TransactionId::new(id))
            .unwrap()
    };

    // A decimal amount, both transaction links (to itself), no `enshrined`.
    let opening = by_id(1910008228);
    assert_eq!(opening.amount, Some(eur(292894)));
    assert_eq!(opening.enshrined, None);
    assert_eq!(
        opening.source_transaction,
        Some(ObjectRef::new(TransactionId::new(1910008228)))
    );
    assert_eq!(
        opening.target_transaction,
        Some(ObjectRef::new(TransactionId::new(1910008228)))
    );
    assert_eq!(
        opening.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert!(opening.create.is_some() && opening.update.is_some());

    // A whole amount and an enshrined transaction.
    let enshrined = by_id(1909988173);
    assert_eq!(enshrined.amount, Some(eur(1000)));
    assert_eq!(enshrined.status, Some(TransactionStatus::Booked));
    assert_eq!(enshrined.enshrined.unwrap().offset().whole_hours(), 1);
    assert_eq!(enshrined.source_transaction, None);
    assert_eq!(enshrined.deleted_at, None);
}

#[test]
fn transaction_reads_create_update_and_client() {
    let t = sample_positions::<Transaction>("transaction")
        .pop()
        .unwrap();
    assert_eq!(t.create.unwrap().offset().whole_hours(), 2);
    assert_eq!(
        t.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
}

#[test]
fn sample_transaction_log_list_reads_every_measured_form() {
    let list = sample_positions::<TransactionLogEntry>("sample_transaction_log_list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, TransactionLogId::new(237319967));
    assert_eq!(
        list[0].object,
        Some(BookedDocument::Invoice(InvoiceId::new(135169196)))
    );
    assert_eq!(
        list[0].sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert!(list[0].create.is_some());
    assert_eq!(list[0].from_status, Some(TransactionStatus::Created));
    assert_eq!(list[0].to_status, Some(TransactionStatus::Booked));
}

#[test]
fn booked_document_reads_the_credit_note_and_an_unknown_object() {
    let credit: BookedDocument =
        serde_json::from_str(r#"{"id":"7","objectName":"CreditNote"}"#).unwrap();
    assert_eq!(credit, BookedDocument::CreditNote(CreditNoteId::new(7)));
    // Synthetic: no measurement shows another object name.
    let other: BookedDocument = serde_json::from_str(r#"{"id":"8","objectName":"Order"}"#).unwrap();
    assert_eq!(
        other,
        BookedDocument::Other {
            object_name: "Order".into(),
            id: 8
        }
    );
}

#[test]
fn transaction_status_350_is_a_spec_only_variant() {
    // Synthetic: 350 is in the OpenAPI response spec only, never measured.
    assert_eq!(
        TransactionStatus::from_code(350),
        TransactionStatus::Unknown350
    );
    assert_eq!(
        TransactionStatus::from_code(999),
        TransactionStatus::Other(999)
    );
}
