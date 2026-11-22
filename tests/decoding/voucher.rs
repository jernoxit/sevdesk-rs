//! Fixture decoding of vouchers.

use super::{eur, serving};
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
