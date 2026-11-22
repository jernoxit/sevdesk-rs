//! Checks 5 and 6: the writable properties of the spec's request schemas
//! against the keys the crate's request types send.
//!
//! The sent keys come from serializing a request built with EVERY option set
//! (and every `*Update` field set) through the crate's public types and
//! collecting the JSON keys at the object that corresponds to the schema.
//! Writable means: a property without `readOnly: true`.
//!
//! A new crate option must be set in the builders below, or it shows up as a
//! writable property that is not sent.

use super::request_samples::*;
use super::{exact_with_hint, writable_properties};
use serde_json::Value as Json;
use serde_yaml_ng::Value;
use sevdesk::*;
use std::collections::BTreeSet;

/// `schema.property`: writable in the spec, sent by no request type. Inline
/// request bodies are named `METHOD path`.
pub(super) const SPEC_WRITABLE_NOT_SENT: &[(&str, &str)] = &[
    (
        "Model_CheckAccountUpdate.accountingNumber",
        "unmeasured; not offered",
    ),
    (
        "Model_CheckAccountUpdate.autoMapTransactions",
        "spec typo: sevDesk ignores it, autoMapTransaction (singular) is sent",
    ),
    ("Model_CheckAccountUpdate.bic", "unmeasured; not offered"),
    (
        "Model_CheckAccountUpdate.defaultAccount",
        "unmeasured; not offered",
    ),
    ("Model_CheckAccountUpdate.iban", "unmeasured; not offered"),
    (
        "Model_CommunicationWayUpdate.contact",
        "same value as stored in the measurement (2026-11-22): no statement",
    ),
    (
        "Model_Contact.defaultDiscountPercentage",
        "same value as stored in the measurement (2026-11-22): no statement",
    ),
    (
        "Model_ContactAddress.name2",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_ContactAddress.name3",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_ContactAddress.name4",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_ContactAddressUpdate.contact",
        "same value as stored in the measurement (2026-11-22): no statement",
    ),
    (
        "Model_ContactAddressUpdate.name2",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_ContactAddressUpdate.name3",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_ContactAddressUpdate.name4",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_ContactUpdate.defaultDiscountPercentage",
        "same value as stored in the measurement (2026-11-22): no statement",
    ),
    (
        "Model_Invoice.id",
        "not offered: saving an existing draft through the factory assigns it a number (measured 2026-11-22); the crate only creates",
    ),
    (
        "Model_Invoice.origin",
        "untestable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "Model_Invoice.taxType",
        "ignored by sevDesk (measured 2026-11-22); the \"Update 1.0\" tax model, taxRule is the crate's",
    ),
    (
        "Model_InvoicePos.discount",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    ("Model_InvoicePos.id", "not offered: the crate only creates"),
    (
        "Model_InvoicePos.part",
        "untestable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "Model_InvoicePos.priceGross",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_InvoicePos.priceTax",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_Voucher.costCentre",
        "untestable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "Model_Voucher.propertyExchangeRate",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_Voucher.propertyForeignCurrencyDeadline",
        "changes other fields (showNet, sums) instead of itself (measured 2026-11-22): not usable",
    ),
    (
        "Model_Voucher.taxSet",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_Voucher.taxType",
        "ignored by sevDesk (measured 2026-11-22); the \"Update 1.0\" tax model, taxRule is the crate's",
    ),
    ("Model_creditNote.id", "not offered: the crate only creates"),
    (
        "Model_creditNote.taxType",
        "ignored by sevDesk (measured 2026-11-22); the \"Update 1.0\" tax model, taxRule is the crate's",
    ),
    (
        "Model_creditNotePos.creditNote",
        "same value as stored in the measurement (2026-11-22): no statement",
    ),
    (
        "Model_creditNotePos.discount",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_creditNotePos.optional",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_creditNotePos.part",
        "untestable: the test account has no such object (measured 2026-11-22)",
    ),
    (
        "Model_creditNotePos.priceGross",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "Model_creditNotePos.priceTax",
        "ignored by sevDesk (measured 2026-11-22)",
    ),
    (
        "POST /CreditNote/{creditNoteId}/sendViaEmail.bccEmail",
        "unmeasured, like the call itself; not offered",
    ),
    (
        "POST /CreditNote/{creditNoteId}/sendViaEmail.ccEmail",
        "unmeasured, like the call itself; not offered",
    ),
    (
        "POST /Invoice/{invoiceId}/sendViaEmail.bccEmail",
        "unmeasured, like the call itself; not offered",
    ),
    (
        "POST /Invoice/{invoiceId}/sendViaEmail.ccEmail",
        "unmeasured, like the call itself; not offered",
    ),
    (
        "PUT /CreditNote/{creditNoteId}/bookAmount.createFeed",
        "not measured; not offered",
    ),
    (
        "PUT /Invoice/{invoiceId}/bookAmount.createFeed",
        "not measured; not offered",
    ),
    (
        "PUT /Voucher/{voucherId}/bookAmount.createFeed",
        "not measured; not offered",
    ),
    (
        "saveCreditNote.discountDelete",
        "not measured; document discounts are not offered",
    ),
    (
        "saveCreditNote.discountSave",
        "not measured; document discounts are not offered",
    ),
    (
        "saveInvoice.discountDelete",
        "not measured; document discounts are not offered",
    ),
    (
        "saveInvoice.discountSave",
        "not measured; document discounts are not offered",
    ),
    (
        "saveInvoice.filename",
        "the upload name of a voucher document; not offered for invoices (unmeasured)",
    ),
    (
        "saveVoucher.voucherPosDelete",
        "not offered: deleting positions is not typed (unmeasured)",
    ),
];

/// `schema.property`: sent by a request type, not a writable property of the
/// spec (absent, or `readOnly`).
pub(super) const SENT_NOT_IN_SPEC: &[(&str, &str)] = &[
    (
        "Model_CheckAccountTransactionUpdate.payeePayerAcctNo",
        "not in the spec; stored (measured 2026-11-22)",
    ),
    (
        "Model_CheckAccountTransactionUpdate.payeePayerBankCode",
        "not in the spec; stored (measured 2026-11-22)",
    ),
    (
        "Model_CheckAccountUpdate.autoMapTransaction",
        "the field is singular; the spec's autoMapTransactions is ignored",
    ),
    (
        "Model_Invoice.addressCity",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_Invoice.addressName",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_Invoice.addressStreet",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_Invoice.addressZip",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_Voucher.id",
        "readOnly in the spec; sent to update an existing draft (measured)",
    ),
    (
        "Model_VoucherPos.id",
        "readOnly in the spec; sent to update an existing position, every position needs its id (measured)",
    ),
    (
        "Model_creditNote.addressCity",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_creditNote.addressName",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_creditNote.addressStreet",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_creditNote.addressZip",
        "not in the spec (only the multi-line address); measured effective, see the address protocol",
    ),
    (
        "Model_creditNote.creditNoteType",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "Model_creditNote.deliveryDate",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "Model_creditNote.deliveryDateUntil",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "Model_creditNote.discount",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "Model_creditNote.paymentMethod",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "Model_creditNote.propertyIsEInvoice",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "Model_creditNote.refSrcInvoice",
        "not in the spec's credit note schema; sent as measured on the test account",
    ),
    (
        "POST /CreditNote/{creditNoteId}/sendViaEmail.sendXml",
        "not in the spec's credit note mail body; unmeasured, like the call",
    ),
];

/// One spec request schema and where its object sits in the serialized
/// request: `pointer` is a JSON pointer, every sample is merged.
struct Case {
    schema: &'static str,
    pointer: &'static str,
    samples: Vec<Json>,
}

fn case(schema: &'static str, pointer: &'static str, samples: Vec<Json>) -> Case {
    Case {
        schema,
        pointer,
        samples,
    }
}

fn cases() -> Vec<Case> {
    let invoice = json(&save_invoice());
    let credit_note = json(&save_credit_note());
    let voucher = json(&save_voucher());
    let send_by = json(&SendBy::FINALIZE);
    let mail = json(&send_via_email());
    let doc_booking = json(&book_document_amount());
    vec![
        case("saveInvoice", "", vec![invoice.clone()]),
        case("Model_Invoice", "/invoice", vec![invoice.clone()]),
        case("Model_InvoicePos", "/invoicePosSave/0", vec![invoice]),
        case("saveCreditNote", "", vec![credit_note.clone()]),
        case("Model_creditNote", "/creditNote", vec![credit_note.clone()]),
        case(
            "Model_creditNotePos",
            "/creditNotePosSave/0",
            vec![credit_note],
        ),
        case("saveVoucher", "", vec![voucher.clone()]),
        case("Model_Voucher", "/voucher", vec![voucher.clone()]),
        case("Model_VoucherPos", "/voucherPosSave/0", vec![voucher]),
        case(
            "Model_CheckAccountTransaction",
            "",
            vec![json(&new_transaction())],
        ),
        case(
            "Model_CheckAccountTransactionUpdate",
            "",
            vec![json(&transaction_update())],
        ),
        case(
            "Model_CheckAccountUpdate",
            "",
            vec![json(&CheckAccountUpdate {
                name: text(),
                auto_map_transaction: Some(true),
            })],
        ),
        case(
            "createFileImportAccount",
            "",
            vec![json(&file_import_account())],
        ),
        case(
            "Model_Contact",
            "",
            vec![
                json(&new_contact(ContactName::Organisation { name: "x".into() })),
                json(&new_contact(ContactName::Person {
                    surename: "x".into(),
                    familyname: "x".into(),
                })),
            ],
        ),
        case("Model_ContactUpdate", "", vec![json(&contact_update())]),
        case(
            "Model_ContactAddress",
            "",
            vec![json(&new_contact_address())],
        ),
        case(
            "Model_ContactAddressUpdate",
            "",
            vec![json(&contact_address_update())],
        ),
        case(
            "Model_CommunicationWay",
            "",
            vec![json(&NewCommunicationWay::email(
                ContactId::new(1),
                "x",
                CommunicationWayKeyId::new(1),
                true,
            ))],
        ),
        case(
            "Model_CommunicationWayUpdate",
            "",
            vec![json(&communication_way_update())],
        ),
        case(
            "Model_ChangeLayout",
            "",
            vec![json(&DocumentParameter::Language(DocumentLanguage::De))],
        ),
        case("PUT /Invoice/{invoiceId}/sendBy", "", vec![send_by.clone()]),
        case("PUT /CreditNote/{creditNoteId}/sendBy", "", vec![send_by]),
        case(
            "POST /Invoice/{invoiceId}/sendViaEmail",
            "",
            vec![mail.clone()],
        ),
        case(
            "POST /CreditNote/{creditNoteId}/sendViaEmail",
            "",
            vec![mail],
        ),
        case(
            "PUT /Invoice/{invoiceId}/bookAmount",
            "",
            vec![doc_booking.clone()],
        ),
        case(
            "PUT /CreditNote/{creditNoteId}/bookAmount",
            "",
            vec![doc_booking],
        ),
        case(
            "PUT /Voucher/{voucherId}/bookAmount",
            "",
            vec![json(&book_voucher_amount())],
        ),
    ]
}

/// The keys of the object at `pointer` over all samples.
fn sent_keys(case: &Case) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for sample in &case.samples {
        let object = sample
            .pointer(case.pointer)
            .and_then(Json::as_object)
            .unwrap_or_else(|| panic!("{}: no object at {:?}", case.schema, case.pointer));
        keys.extend(object.keys().cloned());
    }
    keys
}

pub(super) fn problems(spec: &Value) -> Vec<String> {
    let mut not_sent = BTreeSet::new();
    let mut not_in_spec = BTreeSet::new();
    for case in cases() {
        let writable = writable_properties(spec, case.schema);
        let sent = sent_keys(&case);
        not_sent.extend(
            writable
                .difference(&sent)
                .map(|p| format!("{}.{p}", case.schema)),
        );
        not_in_spec.extend(
            sent.difference(&writable)
                .map(|p| format!("{}.{p}", case.schema)),
        );
    }
    let mut out = Vec::new();
    out.extend(exact_with_hint(
        "SPEC_WRITABLE_NOT_SENT",
        &not_sent,
        SPEC_WRITABLE_NOT_SENT,
        "a writable property the crate does not send: if a write measurement shows \
         sevDesk stores it, add it to the request type (option) and to the builders in \
         request_fields.rs; otherwise list it with the reason (ignored, refused, \
         untestable, spec typo)",
    ));
    out.extend(exact_with_hint(
        "SENT_NOT_IN_SPEC",
        &not_in_spec,
        SENT_NOT_IN_SPEC,
        "a key the crate sends that the spec lacks (or marks readOnly): list it with the \
         measurement that shows it works",
    ));
    out
}
