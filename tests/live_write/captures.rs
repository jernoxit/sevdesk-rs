//! Records the raw write bodies as fixtures for the transport tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helper code outside #[test] functions: a failure here is a test failure"
)]

use super::harness::*;
use reqwest::Method;
use sevdesk::*;
use std::time::Duration;
use time::macros::datetime;

/// Records the raw bodies of the write calls to `target/tmp/captures/` (to
/// refresh the write fixtures) while asserting each answers 2xx. Bodies are
/// built from the typed payloads.
#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_6_record_raw_write_bodies() {
    run("capture", |ctx| async move {
        let at = datetime!(2026-09-29 12:00 +2);
        let json = |v: &dyn erased::Ser| v.to_json();
        let ok = |(status, text): (u16, String)| {
            assert!(status < 300, "{status} {text}");
            text
        };
        let body = ok(ctx
            .raw(
                Method::POST,
                "/CheckAccountTransaction",
                Some(json(&ctx.new_transaction(ZEIT, -45, at))),
                Some("write_transaction_create"),
            )
            .await);
        let tx: serde_json::Value = serde_json::from_str(&body).unwrap();
        let tx_id: i64 = tx["objects"]["id"].as_str().unwrap().parse().unwrap();
        ok(ctx
            .raw(
                Method::PUT,
                &format!("/CheckAccountTransaction/{tx_id}"),
                Some(json(&TransactionUpdate {
                    status: Some(TransactionStatus::Created),
                    ..Default::default()
                })),
                Some("write_transaction_update"),
            )
            .await);

        let pdf = reqwest::multipart::Part::bytes(tiny_pdf())
            .file_name("fee.pdf")
            .mime_str("application/pdf")
            .unwrap();
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let upload = ctx
            .http
            .post(format!("{DEFAULT_BASE_URL}/Voucher/Factory/uploadTempFile"))
            .multipart(reqwest::multipart::Form::new().part("file", pdf))
            .send()
            .await
            .unwrap();
        let upload = upload.text().await.unwrap();
        std::fs::create_dir_all(format!("{}/captures", env!("CARGO_TARGET_TMPDIR"))).unwrap();
        std::fs::write(
            format!(
                "{}/captures/write_upload_temp_file.json",
                env!("CARGO_TARGET_TMPDIR")
            ),
            &upload,
        )
        .unwrap();
        let filename =
            serde_json::from_str::<serde_json::Value>(&upload).unwrap()["objects"]["filename"]
                .as_str()
                .unwrap()
                .to_owned();

        let save = |voucher: VoucherDraft, positions, filename| SaveVoucher {
            voucher,
            positions,
            filename,
        };
        let created = ok(ctx
            .raw(
                Method::POST,
                "/Voucher/Factory/saveVoucher",
                Some(json(&save(
                    ctx.draft(),
                    vec![position(35, "fee"), position(10, "fee2")],
                    None,
                ))),
                Some("write_voucher_save_create"),
            )
            .await);
        let created: serde_json::Value = serde_json::from_str(&created).unwrap();
        let vid: i64 = created["objects"]["voucher"]["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let pids: Vec<i64> = created["objects"]["voucherPos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap().parse().unwrap())
            .collect();
        let mut voucher = ctx.draft();
        voucher.id = Some(VoucherId::new(vid));
        voucher.status = VoucherStatus::Open;
        let positions = vec![
            position(35, "fee").with_id(VoucherPositionId::new(pids[0])),
            position(10, "fee2").with_id(VoucherPositionId::new(pids[1])),
        ];
        ok(ctx
            .raw(
                Method::POST,
                "/Voucher/Factory/saveVoucher",
                Some(json(&save(voucher, positions, Some(filename)))),
                Some("write_voucher_save_update_release_attach"),
            )
            .await);
        ok(ctx
            .raw(
                Method::PUT,
                &format!("/Voucher/{vid}/bookAmount"),
                Some(json(&BookVoucherAmount {
                    amount: eur(-45),
                    date: at,
                    booking_type: BookingType::N,
                    check_account: ObjectRef::new(account(ZEIT)),
                    check_account_transaction: ObjectRef::new(TransactionId::new(tx_id)),
                })),
                Some("write_voucher_book_amount"),
            )
            .await);
        ok(ctx
            .raw(
                Method::PUT,
                &format!("/Voucher/{vid}/resetToOpen"),
                None,
                Some("write_voucher_reset_to_open"),
            )
            .await);
        ok(ctx
            .raw(
                Method::DELETE,
                &format!("/CheckAccountTransaction/{tx_id}"),
                None,
                Some("write_transaction_delete"),
            )
            .await);
        ctx.back_to_draft(VoucherId::new(vid)).await;
        ok(ctx
            .raw(
                Method::DELETE,
                &format!("/Voucher/{vid}"),
                None,
                Some("write_voucher_delete"),
            )
            .await);
    })
    .await;
}

/// Serialising a typed payload to the JSON string the raw helper sends.
mod erased {
    pub trait Ser {
        fn to_json(&self) -> String;
    }
    impl<T: serde::Serialize> Ser for T {
        fn to_json(&self) -> String {
            serde_json::to_string(self).unwrap()
        }
    }
}
