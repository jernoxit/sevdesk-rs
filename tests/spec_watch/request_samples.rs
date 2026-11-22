//! Request builders for `request_fields`: every option and every `*Update`
//! field set, so the serialized JSON carries every key the crate can send.

use serde::Serialize;
use serde_json::Value as Json;
use sevdesk::*;
use time::macros::{date, datetime};
use time::{Date, OffsetDateTime};

pub(super) fn at() -> OffsetDateTime {
    datetime!(2026-11-22 12:00 UTC)
}

pub(super) fn day() -> Date {
    date!(2026 - 11 - 22)
}

pub(super) fn amount() -> Amount {
    Amount::from_minor(1234)
}

pub(super) fn text() -> Option<String> {
    Some("x".into())
}

pub(super) fn json(value: &impl Serialize) -> Json {
    serde_json::to_value(value).expect("request serializes")
}

pub(super) fn position() -> DocumentPositionDraft {
    let mut p = DocumentPositionDraft {
        quantity: 1,
        price: amount(),
        name: "x".into(),
        unity: UnityId::new(1),
        tax_rate_bp: 1900,
        options: PositionOptions::default(),
    };
    p.options.text = text();
    p.options.position_number = Some(1);
    p
}

#[allow(deprecated, reason = "the deprecated options are still sent")]
pub(super) fn save_invoice() -> SaveInvoice {
    let mut draft = InvoiceDraft {
        contact: ObjectRef::new(ContactId::new(1)),
        contact_person: ObjectRef::new(SevUserId::new(1)),
        invoice_date: day(),
        delivery_date: at(),
        header: "x".into(),
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "x".into(),
        currency: CurrencyCode::EUR,
        show_net: true,
        time_to_pay: 14,
        customer_internal_note: "x".into(),
        options: InvoiceOptions::default(),
    };
    let o = &mut draft.options;
    o.delivery_date_until = Some(at());
    o.head_text = text();
    o.foot_text = text();
    o.address_name = text();
    o.address_street = text();
    o.address_zip = text();
    o.address_city = text();
    o.address_country = Some(ObjectRef::new(CountryId::new(1)));
    o.payment_method = Some(ObjectRef::new(PaymentMethodId::new(1)));
    o.is_e_invoice = true;
    o.address = text();
    o.small_settlement = Some(true);
    o.tax_set = Some(ObjectRef::new(TaxSetId::new(1)));
    o.send_date = Some(day());
    o.pay_date = Some(day());
    o.send_type = Some(SendType::Vpdf);
    o.invoice_number = text();
    SaveInvoice::new(draft, vec![position()])
}

#[allow(deprecated, reason = "the deprecated options are still sent")]
pub(super) fn save_credit_note() -> SaveCreditNote {
    let mut draft = CreditNoteDraft {
        credit_note_number: "x".into(),
        contact: ObjectRef::new(ContactId::new(1)),
        contact_person: ObjectRef::new(SevUserId::new(1)),
        credit_note_date: day(),
        delivery_date: at(),
        header: "x".into(),
        tax_rule: ObjectRef::new(TaxRuleId::new(1)),
        tax_text: "x".into(),
        booking_category: CreditNoteCategory::Provision,
        currency: CurrencyCode::EUR,
        show_net: true,
        customer_internal_note: "x".into(),
        options: CreditNoteOptions::default(),
    };
    let o = &mut draft.options;
    o.delivery_date_until = Some(at());
    o.head_text = text();
    o.foot_text = text();
    o.ref_src_invoice = Some(ObjectRef::new(InvoiceId::new(1)));
    o.address_name = text();
    o.address_street = text();
    o.address_zip = text();
    o.address_city = text();
    o.address_country = Some(ObjectRef::new(CountryId::new(1)));
    o.payment_method = Some(ObjectRef::new(PaymentMethodId::new(1)));
    o.is_e_invoice = true;
    o.address = text();
    o.small_settlement = Some(true);
    o.tax_set = Some(ObjectRef::new(TaxSetId::new(1)));
    o.send_date = Some(day());
    o.send_type = Some(SendType::Vpdf);
    SaveCreditNote::new(draft, vec![position()])
}

pub(super) fn save_voucher() -> SaveVoucher {
    let mut draft = VoucherDraft::new(day(), "x", "x", TaxRuleId::new(1), CurrencyCode::EUR);
    let o = &mut draft.options;
    o.id = Some(VoucherId::new(1));
    o.delivery_date = Some(day());
    o.delivery_date_until = Some(day());
    o.supplier = Some(ObjectRef::new(ContactId::new(1)));
    o.payment_deadline = Some(day());
    o.document = Some(ObjectRef::new(DocumentId::new(1)));
    o.pay_date = Some(day());
    let mut p = VoucherPositionDraft::new(AccountDatevId::new(1), 1900, amount(), "x");
    let o = &mut p.options;
    o.id = Some(VoucherPositionId::new(1));
    o.accounting_type = Some(ObjectRef::new(AccountingTypeId::new(1)));
    o.is_asset = Some(true);
    o.sum_gross = Some(amount());
    SaveVoucher {
        voucher: draft,
        positions: vec![p],
        filename: text(),
    }
}

pub(super) fn new_transaction() -> NewTransaction {
    let mut new = NewTransaction {
        value_date: at(),
        amount: amount(),
        payee_payer_name: "x".into(),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(CheckAccountId::new(1)),
        options: NewTransactionOptions::default(),
    };
    let o = &mut new.options;
    o.entry_date = Some(at());
    o.paymt_purpose = text();
    o.payee_payer_acct_no = text();
    o.payee_payer_bank_code = text();
    o.source_transaction = Some(ObjectRef::new(TransactionId::new(1)));
    o.target_transaction = Some(ObjectRef::new(TransactionId::new(2)));
    new
}

pub(super) fn transaction_update() -> TransactionUpdate {
    TransactionUpdate {
        status: Some(TransactionStatus::Created),
        source_transaction: Some(ObjectRef::new(TransactionId::new(1))),
        target_transaction: Some(ObjectRef::new(TransactionId::new(2))),
        value_date: Some(at()),
        entry_date: Some(at()),
        paymt_purpose: text(),
        amount: Some(amount()),
        payee_payer_name: text(),
        payee_payer_acct_no: text(),
        payee_payer_bank_code: text(),
        check_account: Some(ObjectRef::new(CheckAccountId::new(1))),
    }
}

pub(super) fn file_import_account() -> CreateFileImportAccount {
    let mut new = CreateFileImportAccount {
        name: "x".into(),
        import_type: ImportType::Csv,
        options: FileImportAccountOptions::default(),
    };
    new.options.accounting_number = Some(1);
    new.options.iban = text();
    new
}

pub(super) fn new_contact(name: ContactName) -> NewContact {
    let mut new = NewContact::new(name);
    let o = &mut new.options;
    o.buyer_reference = text();
    o.vat_number = text();
    o.customer_number = text();
    o.status = Some(ContactStatus::Active);
    o.parent = Some(ObjectRef::new(ContactId::new(1)));
    o.titel = text();
    o.description = text();
    o.academic_title = text();
    o.gender = text();
    o.name2 = text();
    o.birthday = Some(day());
    o.bank_account = text();
    o.bank_number = text();
    o.default_cashback_time = Some(1);
    o.default_cashback_percent_bp = Some(100);
    o.default_time_to_pay = Some(1);
    o.tax_number = text();
    o.tax_office = text();
    o.exempt_vat = Some(true);
    o.default_discount_amount = Some(amount());
    o.government_agency = Some(true);
    new
}

pub(super) fn contact_update() -> ContactUpdate {
    ContactUpdate {
        name: text(),
        surename: text(),
        familyname: text(),
        buyer_reference: text(),
        vat_number: text(),
        customer_number: text(),
        category: Some(ObjectRef::new(ContactCategoryId::new(1))),
        status: Some(ContactStatus::Active),
        parent: Some(ObjectRef::new(ContactId::new(1))),
        titel: text(),
        description: text(),
        academic_title: text(),
        gender: text(),
        name2: text(),
        birthday: Some(day()),
        bank_account: text(),
        bank_number: text(),
        default_cashback_time: Some(1),
        default_cashback_percent_bp: Some(100),
        default_time_to_pay: Some(1),
        tax_number: text(),
        tax_office: text(),
        exempt_vat: Some(true),
        default_discount_amount: Some(amount()),
        government_agency: Some(true),
    }
}

pub(super) fn new_contact_address() -> NewContactAddress {
    let mut new = NewContactAddress {
        contact: ObjectRef::new(ContactId::new(1)),
        street: "x".into(),
        zip: "x".into(),
        city: "x".into(),
        country: ObjectRef::new(CountryId::new(1)),
        category: ObjectRef::new(AddressCategoryId::new(1)),
        options: NewContactAddressOptions::default(),
    };
    new.options.name = text();
    new
}

pub(super) fn contact_address_update() -> ContactAddressUpdate {
    ContactAddressUpdate {
        street: text(),
        zip: text(),
        city: text(),
        country: Some(ObjectRef::new(CountryId::new(1))),
        category: Some(ObjectRef::new(AddressCategoryId::new(1))),
        name: text(),
    }
}

pub(super) fn communication_way_update() -> CommunicationWayUpdate {
    CommunicationWayUpdate {
        value: text(),
        key: Some(ObjectRef::new(CommunicationWayKeyId::new(1))),
        main: Some(true),
        kind: Some(CommunicationWayType::Email),
    }
}

pub(super) fn send_via_email() -> SendViaEmail {
    let mut mail = SendViaEmail {
        to_email: "x".into(),
        subject: "x".into(),
        text: "x".into(),
        copy: true,
        options: SendViaEmailOptions::default(),
    };
    mail.options.additional_attachments = text();
    mail.options.send_xml = true;
    mail
}

pub(super) fn book_document_amount() -> BookDocumentAmount {
    BookDocumentAmount {
        amount: amount(),
        date: at(),
        booking_type: BookingType::MonetaryTrafficCosts,
        check_account: ObjectRef::new(CheckAccountId::new(1)),
        check_account_transaction: ObjectRef::new(TransactionId::new(1)),
    }
}

pub(super) fn book_voucher_amount() -> BookVoucherAmount {
    BookVoucherAmount {
        amount: amount(),
        date: at(),
        booking_type: BookingType::MonetaryTrafficCosts,
        check_account: ObjectRef::new(CheckAccountId::new(1)),
        check_account_transaction: ObjectRef::new(TransactionId::new(1)),
    }
}
