//! Transaction writes: round trip and the transfer link between accounts.

use super::harness::*;
use sevdesk::*;
use time::macros::datetime;

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_1_transaction_roundtrip() {
    run("tx", |ctx| async move {
        let sent = datetime!(2026-09-29 12:00 +2);
        let created = ctx
            .c
            .create_transaction(&ctx.new_transaction(ZEIT, -35, sent))
            .await
            .unwrap();
        let read = ctx.c.transaction(created.id).await.unwrap();
        assert_eq!(read.amount, Some(eur(-35)));
        assert_eq!(read.value_date, Some(sent));
        assert_eq!(
            read.value_date.unwrap().offset().whole_hours(),
            2,
            "offset kept"
        );
        assert_eq!(read.paymt_purpose.as_deref(), Some(ctx.marker.as_str()));
        assert_eq!(read.status, Some(TransactionStatus::Created));
        assert_eq!(read.check_account.unwrap().id, account(ZEIT));

        // The filter is a substring match: compare exactly afterwards.
        let mut q = TransactionQuery::new(account(ZEIT));
        q.paymt_purpose = Some(ctx.marker.clone());
        let found: Vec<_> = ctx
            .c
            .list_transactions(&q)
            .await
            .unwrap()
            .into_iter()
            .filter(|t| t.paymt_purpose.as_deref() == Some(ctx.marker.as_str()))
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, created.id);
    })
    .await;
}

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_4_transfer_link_between_accounts() {
    run("transfer", |ctx| async move {
        let at = datetime!(2026-09-29 12:00 +2);
        let payout = ctx
            .c
            .create_transaction(&ctx.new_transaction(ZEIT, -100, at))
            .await
            .unwrap();
        let receipt = ctx
            .c
            .create_transaction(&ctx.new_transaction(BANK, 100, at))
            .await
            .unwrap();
        ctx.c
            .update_transaction(
                payout.id,
                &TransactionUpdate {
                    status: Some(TransactionStatus::Booked),
                    target_transaction: Some(ObjectRef::new(receipt.id)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        ctx.c
            .update_transaction(
                receipt.id,
                &TransactionUpdate {
                    status: Some(TransactionStatus::Booked),
                    source_transaction: Some(ObjectRef::new(payout.id)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let payout = ctx.c.transaction(payout.id).await.unwrap();
        let receipt = ctx.c.transaction(receipt.id).await.unwrap();
        // The link is stored only on the side it is set on (measured).
        assert_eq!(payout.target_transaction.unwrap().id, receipt.id);
        assert!(payout.source_transaction.is_none());
        assert_eq!(receipt.source_transaction.unwrap().id, payout.id);
        assert!(receipt.target_transaction.is_none());
        assert_eq!(payout.status, Some(TransactionStatus::Booked));
        assert_eq!(receipt.status, Some(TransactionStatus::Booked));
    })
    .await;
}
