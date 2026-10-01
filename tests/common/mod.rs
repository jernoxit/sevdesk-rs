#![allow(
    dead_code,
    clippy::expect_used,
    reason = "each test crate that includes this module uses only some of the helpers; a failed setup is a test failure"
)]

pub mod scrub;

use sevdesk::Amount;
use sevdesk::{
    ApiToken, CheckAccountId, ContactId, CreditNoteCategory, CreditNoteDraft, CurrencyCode,
    DocumentPositionDraft, InvoiceDraft, NewTransaction, ObjectRef, Pacing, SevUserId,
    SevdeskClient, SevdeskClientConfig, TaxRuleId, TransactionStatus, UnityId,
};
use time::macros::{date, datetime};
use wiremock::MockServer;

/// A token that is obviously not a real one.
pub const TEST_TOKEN: &str = "test-token-0000";

pub fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {path}: {e}"))
}

pub fn client_for(server: &MockServer, pacing: Pacing) -> SevdeskClient {
    SevdeskClient::new(SevdeskClientConfig {
        base_url: server.uri(),
        api_token: ApiToken::new(TEST_TOKEN),
        pacing,
        x_version: None,
    })
    .expect("client")
}

/// A client with no throttling for a mock server.
pub fn client(server: &MockServer) -> SevdeskClient {
    client_for(server, Pacing::NONE)
}

pub fn eur(minor: i64) -> Amount {
    Amount::from_minor(minor)
}

pub fn new_transaction() -> NewTransaction {
    NewTransaction {
        value_date: datetime!(2026-08-15 12:00 +2),
        entry_date: None,
        amount: eur(1),
        payee_payer_name: "Stripe Payments Europe, Limited".into(),
        paymt_purpose: Some("txn_1".into()),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(CheckAccountId::new(6306017)),
        source_transaction: None,
        target_transaction: None,
    }
}

/// A minimal gross invoice draft: every optional absent.
pub fn invoice_draft(currency: CurrencyCode) -> InvoiceDraft {
    InvoiceDraft {
        contact: ObjectRef::new(ContactId::new(7)),
        contact_person: ObjectRef::new(SevUserId::new(9)),
        invoice_date: date!(2026 - 09 - 29),
        delivery_date: datetime!(2026-09-29 0:00 +2),
        delivery_date_until: None,
        header: "SS-LIVE-1".into(),
        head_text: None,
        foot_text: None,
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "Umsatzsteuer 19%".into(),
        currency,
        show_net: false,
        time_to_pay: 14,
        customer_internal_note: "SS-LIVE-REF-1".into(),
        address_name: None,
        address_street: None,
        address_zip: None,
        address_city: None,
        address_country: None,
        payment_method: None,
        is_e_invoice: false,
    }
}

/// A minimal gross credit note draft without an invoice reference.
pub fn credit_note_draft(number: &str, currency: CurrencyCode) -> CreditNoteDraft {
    CreditNoteDraft {
        credit_note_number: number.into(),
        contact: ObjectRef::new(ContactId::new(7)),
        contact_person: ObjectRef::new(SevUserId::new(9)),
        credit_note_date: date!(2026 - 09 - 29),
        delivery_date: datetime!(2026-09-29 0:00 +2),
        delivery_date_until: None,
        header: "SS-LIVE-2".into(),
        head_text: None,
        foot_text: None,
        tax_rule: ObjectRef::new(TaxRuleId::new(9)),
        tax_text: "Umsatzsteuer 19%".into(),
        booking_category: CreditNoteCategory::Provision,
        ref_src_invoice: None,
        currency,
        show_net: false,
        customer_internal_note: "SS-LIVE-REF-2".into(),
        address_name: None,
        address_street: None,
        address_zip: None,
        address_city: None,
        address_country: None,
        payment_method: None,
        is_e_invoice: false,
    }
}

/// One piece at `price` cents with 19 % tax.
pub fn position(name: &str, price: i64) -> DocumentPositionDraft {
    DocumentPositionDraft {
        quantity: 1,
        price: eur(price),
        name: name.into(),
        text: None,
        unity: UnityId::new(1),
        tax_rate_bp: 1900,
    }
}
