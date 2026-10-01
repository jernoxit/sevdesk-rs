//! Voucher writes: draft, update, attach, release, book, delete.

use super::harness::*;
use reqwest::Method;
use sevdesk::*;
use time::macros::datetime;

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_2_voucher_draft_update_and_attach() {
    run("voucher", |ctx| async move {
        let saved = ctx
            .c
            .save_voucher(&SaveVoucher {
                voucher: ctx.draft(),
                positions: vec![position(35, "fee")],
                filename: None,
            })
            .await
            .unwrap();
        let id = saved.voucher.id;
        let v = &saved.voucher;
        assert_eq!(v.status, Some(VoucherStatus::Draft));
        assert_eq!(v.credit_debit, Some(CreditDebit::Credit));
        assert_eq!(v.voucher_type, Some(VoucherType::Normal));
        assert_eq!(v.tax_rule.unwrap().id, TaxRuleId::new(14));
        assert_eq!(v.description.as_deref(), Some(ctx.marker.as_str()));
        assert_eq!(v.sum_gross, Some(eur(35)));
        assert!(v.document.is_none());
        let p = &saved.voucher_pos[0];
        assert_eq!(p.sum_net, Some(eur(35)));
        assert_eq!(p.account_datev.unwrap().id, AccountDatevId::new(4285));
        assert_eq!(p.tax_rate_bp, Some(0));
        assert_eq!(p.net, Some(true));
        let read = ctx.c.voucher(id).await.unwrap();
        assert_eq!(read.sum_gross, Some(eur(35)));

        // Update: ALL positions with their ids, plus a second one.
        let mut voucher = ctx.draft();
        voucher.id = Some(id);
        let mut positions = with_ids(&saved, &[(35, "fee")]);
        positions.push(position(10, "fee2"));
        let updated = ctx
            .c
            .save_voucher(&SaveVoucher {
                voucher: voucher.clone(),
                positions,
                filename: None,
            })
            .await
            .unwrap();
        assert_eq!(updated.voucher.id, id);
        assert_eq!(updated.voucher.sum_gross, Some(eur(45)));
        assert_eq!(updated.voucher_pos.len(), 2);

        // Attach a PDF: upload, then update with `filename` and all positions.
        let uploaded = ctx
            .c
            .upload_temp_file("fee.pdf", "application/pdf", tiny_pdf())
            .await
            .unwrap();
        assert!(!uploaded.filename.is_empty());
        let attached = ctx
            .c
            .save_voucher(&SaveVoucher {
                voucher,
                positions: with_ids(&updated, &[(35, "fee"), (10, "fee2")]),
                filename: Some(uploaded.filename),
            })
            .await
            .unwrap();
        assert!(attached.voucher.document.is_some());
        assert_eq!(attached.voucher.sum_gross, Some(eur(45)));
        assert_eq!(attached.voucher_pos.len(), 2, "no duplicated positions");
        let read = ctx.c.voucher(id).await.unwrap();
        assert!(read.document.is_some());
        assert_eq!(ctx.c.voucher_positions(id).await.unwrap().len(), 2);
    })
    .await;
}

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_3_release_book_log_reset() {
    run("book", |ctx| async move {
        let saved = ctx
            .c
            .save_voucher(&SaveVoucher {
                voucher: ctx.draft(),
                positions: vec![position(35, "fee"), position(10, "fee2")],
                filename: None,
            })
            .await
            .unwrap();
        let id = saved.voucher.id;
        // Release through the API (the flow leaves it to the human, but it works).
        let mut released = ctx.draft();
        released.id = Some(id);
        released.status = VoucherStatus::Open;
        let saved = ctx
            .c
            .save_voucher(&SaveVoucher {
                voucher: released,
                positions: with_ids(&saved, &[(35, "fee"), (10, "fee2")]),
                filename: None,
            })
            .await
            .unwrap();
        assert_eq!(saved.voucher.status, Some(VoucherStatus::Open));
        assert_eq!(saved.voucher.sum_gross, Some(eur(45)));

        let at = datetime!(2026-09-29 12:00 +2);
        let tx = ctx
            .c
            .create_transaction(&ctx.new_transaction(ZEIT, -45, at))
            .await
            .unwrap();
        ctx.c
            .book_voucher_amount(
                id,
                &BookVoucherAmount {
                    amount: eur(-45),
                    date: at,
                    booking_type: BookingType::N,
                    check_account: ObjectRef::new(account(ZEIT)),
                    check_account_transaction: ObjectRef::new(tx.id),
                },
            )
            .await
            .unwrap();
        let v = ctx.c.voucher(id).await.unwrap();
        assert_eq!(v.status, Some(VoucherStatus::Paid));
        assert_eq!(v.paid_amount, Some(eur(45)));
        let t = ctx.c.transaction(tx.id).await.unwrap();
        assert_eq!(t.status, Some(TransactionStatus::Booked));
        let log = ctx.c.transaction_log(tx.id).await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].object, Some(BookedDocument::Voucher(id)));
        assert_eq!(log[0].amount_paid, Some(eur(-45)));
        assert_eq!(log[0].to_status, Some(TransactionStatus::Booked));

        ctx.c.reset_voucher_to_open(id).await.unwrap();
        let v = ctx.c.voucher(id).await.unwrap();
        assert_eq!(v.status, Some(VoucherStatus::Open));
        assert_eq!(v.paid_amount, Some(eur(0)));
        let t = ctx.c.transaction(tx.id).await.unwrap();
        assert_eq!(t.status, Some(TransactionStatus::Created));
        assert!(ctx.c.transaction_log(tx.id).await.unwrap().is_empty());

        // A released voucher only goes back to a draft through resetToDraft.
        ctx.c.reset_voucher_to_draft(id).await.unwrap();
        assert_eq!(
            ctx.c.voucher(id).await.unwrap().status,
            Some(VoucherStatus::Draft)
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_5_delete_transaction_and_voucher() {
    run("delete", |ctx| async move {
        let at = datetime!(2026-09-29 12:00 +2);
        let tx = ctx
            .c
            .create_transaction(&ctx.new_transaction(ZEIT, -1, at))
            .await
            .unwrap();
        ctx.c.delete_transaction(tx.id).await.unwrap();
        let mut q = TransactionQuery::new(account(ZEIT));
        q.paymt_purpose = Some(ctx.marker.clone());
        assert!(
            ctx.c.list_transactions(&q).await.unwrap().is_empty(),
            "list omits deleted"
        );
        let read = ctx.c.transaction(tx.id).await.unwrap();
        assert!(
            read.is_deleted(),
            "get by id still answers, with deleted_at"
        );

        let saved = ctx
            .c
            .save_voucher(&SaveVoucher {
                voucher: ctx.draft(),
                positions: vec![position(1, "x")],
                filename: None,
            })
            .await
            .unwrap();
        let (status, body) = ctx
            .raw(
                Method::DELETE,
                &format!("/Voucher/{}", saved.voucher.id),
                None,
                None,
            )
            .await;
        assert!(status < 300, "delete voucher: {status} {body}");
        let list = ctx
            .c
            .list_vouchers(&VoucherQuery {
                description_like: Some(ctx.marker.clone()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(list.is_empty(), "list omits the deleted voucher");
    })
    .await;
}
