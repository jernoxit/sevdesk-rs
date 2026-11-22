//! Fixture decoding of vouchers.

use super::{eur, fixture, sample_positions, serving};
use sevdesk::*;

#[tokio::test]
async fn voucher_with_number_paid_amount() {
    let (_s, c) = serving("/Voucher/156244013", "voucher").await;
    let v = c.voucher(VoucherId::new(156244013)).await.unwrap();
    assert_eq!(v.status, Some(VoucherStatus::Paid));
    assert_eq!(v.sum_gross, Some(eur(8749)));
    // `"paidAmount": 87.49` — a JSON number, read without a float.
    assert_eq!(v.paid_amount, Some(eur(8749)));
    assert_eq!(v.currency, CurrencyCode::EUR);
    assert_eq!(v.credit_debit, Some(CreditDebit::Credit));
    assert_eq!(v.voucher_type, Some(VoucherType::Normal));
    assert_eq!(v.tax_rule.unwrap().id, TaxRuleId::new(14));
    assert!(v.document.is_some());
    assert!(v.delivery_date_until.is_none());
    assert_eq!(
        v.supplier_name.as_deref(),
        Some("Stripe Payments Europe, Limited")
    );
}

#[tokio::test]
async fn voucher_positions() {
    let (_s, c) = serving("/VoucherPos", "voucher_positions").await;
    let p = c
        .voucher_positions(VoucherId::new(156244013))
        .await
        .unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].sum_net, Some(eur(5749)));
    // A glatter Betrag arrives as "30".
    assert_eq!(p[1].sum_net, Some(eur(3000)));
    assert_eq!(p[0].tax_rate_bp, Some(0));
    assert_eq!(p[0].account_datev.unwrap().id, AccountDatevId::new(4285));
    assert_eq!(p[0].net, Some(true));
}

#[tokio::test]
async fn voucher_list() {
    let (_s, c) = serving("/Voucher", "voucher_list").await;
    let list = c.list_vouchers(&VoucherQuery::default()).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].status, Some(VoucherStatus::PartiallyPaid));
    assert!(list.iter().all(|v| v.currency == CurrencyCode::EUR));
}

#[tokio::test]
async fn voucher_reads_every_measured_form() {
    let (_s, c) = serving("/Voucher/156244013", "voucher").await;
    let v = c.voucher(VoucherId::new(156244013)).await.unwrap();
    assert_eq!(
        v.create,
        Some(time::macros::datetime!(2026-09-29 15:20:31 +2))
    );
    assert_eq!(
        v.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert_eq!(v.create_user, Some(ObjectRef::new(SevUserId::new(1551095))));
    assert_eq!(v.sum_tax, Some(eur(0)));
    assert_eq!(v.sum_net_accounting, Some(eur(8749)));
    assert_eq!(v.sum_gross_foreign_currency, Some(eur(0)));
    assert_eq!(v.show_net, Some(true));
    assert_eq!(v.hidden, Some(false));
    assert_eq!(v.tip, Some(eur(0)));
    assert_eq!(v.mileage_rate, Some(Decimal::new(0, 0)));
    assert_eq!(
        v.pay_date,
        Some(time::macros::datetime!(2026-08-25 12:00 +2))
    );
    assert_eq!(v.document, Some(ObjectRef::new(DocumentId::new(300881699))));
    // `null` in every measurement: the forms come from the spec.
    assert_eq!(v.tax_type, None);
    assert_eq!(v.recurring_interval, None);
    assert_eq!(v.enshrined, None);
    assert_eq!(
        v.supplier_name_at_save.as_deref(),
        Some("Stripe Payments Europe, Limited")
    );
}

#[test]
fn spec_only_voucher_keys_read_their_spec_types() {
    // Synthetic: no measurement has these keys filled; the forms are the
    // OpenAPI spec's.
    let v: Voucher = serde_json::from_str(
        r#"{"id": "1", "currency": "EUR", "mapAll": true, "supplier": {"id": "7", "objectName": "Contact"},
            "costCentre": {"id": "8", "objectName": "CostCentre"},
            "taxSet": {"id": "9", "objectName": "TaxSet"}, "taxType": "eu",
            "recurringInterval": "P0Y1M0W", "recurringStartDate": "2026-01-01T00:00:00+01:00",
            "sumDiscounts": "1.5", "propertyExchangeRate": "1.08"}"#,
    )
    .unwrap();
    assert_eq!(v.map_all, Some(true));
    assert_eq!(v.supplier, Some(ObjectRef::new(ContactId::new(7))));
    assert_eq!(v.cost_centre, Some(ObjectRef::new(CostCentreId::new(8))));
    assert_eq!(v.tax_set, Some(ObjectRef::new(TaxSetId::new(9))));
    assert_eq!(v.tax_type, Some(TaxType::Eu));
    assert_eq!(v.recurring_interval, Some(RecurringInterval::Monthly));
    assert_eq!(
        v.recurring_start_date,
        Some(time::macros::datetime!(2026-01-01 0:00 +1))
    );
    assert_eq!(v.sum_discounts.as_deref(), Some("1.5"));
    assert_eq!(v.property_exchange_rate.as_deref(), Some("1.08"));
    let other: Voucher = serde_json::from_str(
        r#"{"id": "1", "currency": "EUR", "recurringInterval": "P9Y0M0W", "status": "150"}"#,
    )
    .unwrap();
    assert_eq!(
        other.recurring_interval,
        Some(RecurringInterval::Other("P9Y0M0W".into()))
    );
    assert_eq!(other.status, Some(VoucherStatus::Transferred));
}

#[test]
fn voucher_position_spec_only_references_read() {
    let p: VoucherPosition = serde_json::from_str(
        r#"{"id": "1", "accountingType": {"id": "5", "objectName": "AccountingType"},
            "estimatedAccountingType": {"id": "6", "objectName": "AccountingType"}}"#,
    )
    .unwrap();
    assert_eq!(
        p.accounting_type,
        Some(ObjectRef::new(AccountingTypeId::new(5)))
    );
    assert_eq!(
        p.estimated_accounting_type,
        Some(ObjectRef::new(AccountingTypeId::new(6)))
    );
}

fn read_object<T: serde::de::DeserializeOwned>(name: &str) -> T {
    #[derive(serde::Deserialize)]
    struct Envelope<T> {
        objects: T,
    }
    serde_json::from_str::<Envelope<T>>(&fixture(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .objects
}

#[test]
fn saved_voucher_reads_the_embedded_document() {
    let saved: SavedVoucher = read_object("write_voucher_save_update_release_attach");
    assert_eq!(saved.vatdrop.as_deref(), Some("not relevant"));
    let doc = saved.document.unwrap();
    assert_eq!(doc.id, DocumentId::new(300938451));
    assert_eq!(
        doc.filename.as_deref(),
        Some("d97ad10ac85bdb2d863c4fa9af354997")
    );
    assert_eq!(doc.status, Some(50));
    assert_eq!(doc.extension.as_deref(), Some("pdf"));
    assert_eq!(doc.filesize, Some(560));
    assert_eq!(
        doc.create_user,
        Some(ObjectRef::new(SevUserId::new(1551095)))
    );
    assert_eq!(
        doc.update_user,
        Some(ObjectRef::new(SevUserId::new(1551095)))
    );
    assert_eq!(
        doc.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
    assert_eq!(
        doc.create,
        Some(time::macros::datetime!(2026-09-29 21:19:15 +2))
    );
    assert!(doc.content_hash.is_some());
    assert_eq!(saved.voucher_pos.len(), 2);
}

#[test]
fn voucher_log_reads_the_booking() {
    let log: VoucherLog = read_object("write_voucher_book_amount");
    assert_eq!(log.id, VoucherLogId::new(148586846));
    assert_eq!(
        log.create,
        Some(time::macros::datetime!(2026-09-29 21:19:16 +2))
    );
    assert_eq!(log.voucher, Some(ObjectRef::new(VoucherId::new(156270656))));
    assert_eq!(log.from_status, Some(VoucherStatus::Open));
    assert_eq!(log.to_status, Some(VoucherStatus::Paid));
    assert_eq!(log.amount_payed, Some(eur(45)));
    assert_eq!(
        log.booking_date,
        Some(time::macros::datetime!(2026-09-29 12:00 +2))
    );
    assert_eq!(
        log.sev_client,
        Some(ObjectRef::new(SevClientId::new(1303847)))
    );
}

#[test]
fn sample_voucher_list_reads_every_measured_form() {
    let list = sample_positions::<Voucher>("sample_voucher_list");
    assert_eq!(list.len(), 3);
    let by_id = |id: i64| list.iter().find(|v| v.id == VoucherId::new(id)).unwrap();

    let decimal = by_id(156244013);
    assert_eq!(decimal.sum_gross, Some(eur(8749)));
    assert_eq!(decimal.paid_amount, Some(eur(8749)));
    assert_eq!(
        decimal.document,
        Some(ObjectRef::new(DocumentId::new(300881699)))
    );
    assert_eq!(
        decimal.supplier_name_at_save.as_deref(),
        Some("Stripe Payments Europe, Limited")
    );

    // A whole amount arrives as `"80"` and `80`; no document, no name at save.
    let whole = by_id(156247125);
    assert_eq!(whole.sum_gross, Some(eur(8000)));
    assert_eq!(whole.paid_amount, Some(eur(8000)));
    assert_eq!(whole.document, None);
    assert_eq!(whole.supplier_name_at_save, None);

    let draft = by_id(156242963);
    assert_eq!(draft.status, Some(VoucherStatus::Draft));
    assert_eq!(draft.sum_gross, Some(eur(5499)));
    assert_eq!(draft.paid_amount, Some(eur(0)));
    assert_eq!(draft.pay_date, None);
    assert!(list.iter().all(|v| v.hidden == Some(false)
        && v.selected_for_payment_file == Some(false)
        && v.currency == CurrencyCode::EUR));
}

#[test]
fn sample_voucher_pos_list_reads_every_measured_form() {
    let list = sample_positions::<VoucherPosition>("sample_voucher_pos_list");
    assert_eq!(list.len(), 2);
    let by_id = |id: i64| {
        list.iter()
            .find(|p| p.id == VoucherPositionId::new(id))
            .unwrap()
    };
    let decimal = by_id(213038234);
    assert_eq!(decimal.sum_net, Some(eur(2499)));
    assert_eq!(decimal.sum_net_accounting, Some(eur(2499)));
    assert_eq!(decimal.catering_tip, Some(eur(0)));
    let whole = by_id(213039665);
    assert_eq!(whole.sum_net, Some(eur(3000)));
    assert_eq!(whole.catering_tip, None);
    for p in &list {
        assert_eq!(p.is_gwg, Some(false));
        assert_eq!(p.is_asset, Some(false));
        assert_eq!(p.is_percentage, Some(true));
        assert_eq!(p.discounted_value, Some(Decimal::new(0, 0)));
        assert_eq!(p.asset_memo_value, Some(eur(0)));
        assert_eq!(p.sum_tax_foreign_currency, Some(eur(0)));
        assert!(p.create.is_some() && p.update.is_some());
    }
}
