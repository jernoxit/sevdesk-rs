//! Completeness guard: every key a recorded answer carries is either read by
//! the read type or stands on that fixture's named list of unread keys.
//!
//! No network: each fixture is decoded into its read type the way the client
//! decodes `objects`, collecting every key the type ignores (`serde_ignored`).
//! The lists are compared by EXACT equality: a key that appears (the type lost
//! a field, the fixture changed) fails, and a key that disappears (a field was
//! added) fails too, with the new list in the message so it is lowered in the
//! same commit. List indices are normalized to `*`.

mod common;

use common::fixture;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;
use sevdesk::*;
use std::collections::BTreeSet;

/// The keys of `name` that `T` (or `Vec<T>` for a list answer) does not read.
fn unread<T: DeserializeOwned>(name: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct Envelope {
        objects: Box<RawValue>,
    }
    let envelope: Envelope =
        serde_json::from_str(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let raw = envelope.objects.get();
    let is_list = raw.trim_start().starts_with('[');
    let mut found = BTreeSet::new();
    let mut de = serde_json::Deserializer::from_str(raw);
    let record = |path: serde_ignored::Path<'_>| {
        let segments: Vec<String> = path
            .to_string()
            .split('.')
            .filter(|s| *s != "?")
            .map(|s| {
                if s.bytes().all(|b| b.is_ascii_digit()) {
                    "*".to_owned()
                } else {
                    s.to_owned()
                }
            })
            .collect();
        found.insert(format!("objects.{}", segments.join(".")));
    };
    if is_list {
        serde_ignored::deserialize::<_, _, Vec<T>>(&mut de, record)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    } else {
        serde_ignored::deserialize::<_, _, T>(&mut de, record)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    let embedded = embedded_objects(name);
    found
        .into_iter()
        .map(
            |key| match embedded.iter().find(|p| key.starts_with(&format!("{p}."))) {
                Some(prefix) => format!("{prefix}.<embedded object>"),
                None => key,
            },
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Paths where a recorded answer embeds a FULL object that the read type reads
/// as an `ObjectRef` (another call of the crate returns that object). All its
/// keys but the id are unread by design and collapse to one entry per path.
fn embedded_objects(fixture: &str) -> &'static [&'static str] {
    match fixture {
        "write_invoice_cancel" => &["objects.contact", "objects.taxRule"],
        "write_invoice_save" | "quantity_probe_save" => {
            &["objects.invoicePos.*.invoice", "objects.invoicePos.*.unity"]
        }
        _ => &[],
    }
}

/// Invoice keys that no recording or survey shows filled and that the OpenAPI
/// spec does not type (form unmeasured): not read until a measurement shows
/// them. The flag says whether any fixture carries the key at all.
const INVOICE_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("accountEndDate", true), // not in the spec; null in all 313 survey objects
    ("accountLastInvoice", true), // not in the spec; null in all 313 survey objects
    ("accountStartDate", true), // not in the spec; null in all 313 survey objects
    ("addressGender", true),  // not in the spec; null in all 313 survey objects
    ("addressName2", true),   // not in the spec; null in all 313 survey objects
    ("addressParentName", true), // not in the spec; null in all 313 survey objects
    ("addressParentName2", true), // not in the spec; null in all 313 survey objects
    ("datevConnectOnline", false), // spec: an object without fields; in no recording
    ("propertyIsBookkeepingV1", true), // not in the spec; null in both recordings that have it
    ("sepaCreditorId", true), // not in the spec; null in all 313 survey objects
    ("sepaMandateReference", true), // not in the spec; null in all 313 survey objects
    ("sepaPayerIban", true),  // not in the spec; null in all 313 survey objects
];

/// InvoicePos keys of the same kind.
const INVOICE_POSITION_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 408 objects
];

/// Fixtures whose read type has a form-unmeasured list: the full path of each
/// key of that list in the fixture.
fn form_unmeasured(fixture: &str) -> Vec<(String, bool)> {
    let under = |prefix: &str, keys: &[(&str, bool)]| -> Vec<(String, bool)> {
        keys.iter()
            .map(|(k, recorded)| (format!("{prefix}{k}"), *recorded))
            .collect()
    };
    match fixture {
        "invoice_draft" | "invoice_enshrined" | "invoice_list" => {
            under("objects.*.", INVOICE_FORM_UNMEASURED)
        }
        "write_invoice_cancel" => under("objects.", INVOICE_FORM_UNMEASURED),
        "write_invoice_save" | "quantity_probe_save" => {
            let mut keys = under("objects.invoice.", INVOICE_FORM_UNMEASURED);
            keys.extend(under(
                "objects.invoicePos.*.",
                INVOICE_POSITION_FORM_UNMEASURED,
            ));
            keys
        }
        "quantity_probe_get_invoice_pos" => under("objects.*.", INVOICE_POSITION_FORM_UNMEASURED),
        _ => Vec::new(),
    }
}

/// Fixtures that are read answers, with the read type they decode into and
/// the keys that type does not read yet.
fn covered() -> Vec<(&'static str, Vec<String>, &'static [&'static str])> {
    vec![
        (
            "check_account",
            unread::<CheckAccount>("check_account"),
            &[
                "objects.*.additionalInformation",
                "objects.*.autoSyncTransactions",
                "objects.*.bankServer",
                "objects.*.baseAccount",
                "objects.*.bic",
                "objects.*.checkAccId",
                "objects.*.countryCode",
                "objects.*.create",
                "objects.*.defaultAccount",
                "objects.*.iban",
                "objects.*.importType",
                "objects.*.lastSync",
                "objects.*.objectName",
                "objects.*.pin",
                "objects.*.priority",
                "objects.*.sevClient",
                "objects.*.status",
                "objects.*.translationCode",
                "objects.*.update",
            ],
        ),
        (
            "check_account_list",
            unread::<CheckAccount>("check_account_list"),
            &[
                "objects.*.additionalInformation",
                "objects.*.autoSyncTransactions",
                "objects.*.bankServer",
                "objects.*.baseAccount",
                "objects.*.bic",
                "objects.*.checkAccId",
                "objects.*.countryCode",
                "objects.*.create",
                "objects.*.defaultAccount",
                "objects.*.iban",
                "objects.*.importType",
                "objects.*.lastSync",
                "objects.*.objectName",
                "objects.*.pin",
                "objects.*.priority",
                "objects.*.sevClient",
                "objects.*.status",
                "objects.*.translationCode",
                "objects.*.update",
            ],
        ),
        (
            "communication_way",
            unread::<CommunicationWay>("communication_way"),
            &[
                "objects.additionalInformation",
                "objects.contact.objectName",
                "objects.create",
                "objects.key.objectName",
                "objects.objectName",
                "objects.sevClient",
                "objects.update",
            ],
        ),
        (
            "communication_way_list",
            unread::<CommunicationWay>("communication_way_list"),
            &[
                "objects.*.additionalInformation",
                "objects.*.contact.objectName",
                "objects.*.create",
                "objects.*.key.objectName",
                "objects.*.objectName",
                "objects.*.sevClient",
                "objects.*.update",
            ],
        ),
        (
            "contact",
            unread::<Contact>("contact"),
            &[
                "objects.*.academicTitle",
                "objects.*.additionalInformation",
                "objects.*.bankAccount",
                "objects.*.bankNumber",
                "objects.*.birthday",
                "objects.*.category",
                "objects.*.create",
                "objects.*.defaultCashbackPercent",
                "objects.*.defaultCashbackTime",
                "objects.*.defaultDiscountAmount",
                "objects.*.defaultDiscountPercentage",
                "objects.*.defaultShowVat",
                "objects.*.defaultTimeToPay",
                "objects.*.description",
                "objects.*.exemptVat",
                "objects.*.gender",
                "objects.*.governmentAgency",
                "objects.*.mandateReference",
                "objects.*.name2",
                "objects.*.objectName",
                "objects.*.sevClient",
                "objects.*.status",
                "objects.*.taxNumber",
                "objects.*.taxOffice",
                "objects.*.taxType",
                "objects.*.titel",
                "objects.*.update",
            ],
        ),
        (
            "contact_list_by_customer_number",
            unread::<Contact>("contact_list_by_customer_number"),
            &[
                "objects.*.academicTitle",
                "objects.*.additionalInformation",
                "objects.*.bankAccount",
                "objects.*.bankNumber",
                "objects.*.birthday",
                "objects.*.category",
                "objects.*.create",
                "objects.*.defaultCashbackPercent",
                "objects.*.defaultCashbackTime",
                "objects.*.defaultDiscountAmount",
                "objects.*.defaultDiscountPercentage",
                "objects.*.defaultShowVat",
                "objects.*.defaultTimeToPay",
                "objects.*.description",
                "objects.*.exemptVat",
                "objects.*.gender",
                "objects.*.governmentAgency",
                "objects.*.mandateReference",
                "objects.*.name2",
                "objects.*.objectName",
                "objects.*.sevClient",
                "objects.*.status",
                "objects.*.taxNumber",
                "objects.*.taxOffice",
                "objects.*.taxType",
                "objects.*.titel",
                "objects.*.update",
            ],
        ),
        (
            "contact_address",
            unread::<ContactAddress>("contact_address"),
            &[
                "objects.additionalInformation",
                "objects.category.objectName",
                "objects.contact.academicTitle",
                "objects.contact.additionalInformation",
                "objects.contact.bankAccount",
                "objects.contact.bankNumber",
                "objects.contact.birthday",
                "objects.contact.buyerReference",
                "objects.contact.category",
                "objects.contact.create",
                "objects.contact.customerNumber",
                "objects.contact.defaultCashbackPercent",
                "objects.contact.defaultCashbackTime",
                "objects.contact.defaultDiscountAmount",
                "objects.contact.defaultDiscountPercentage",
                "objects.contact.defaultShowVat",
                "objects.contact.defaultTimeToPay",
                "objects.contact.description",
                "objects.contact.exemptVat",
                "objects.contact.familyname",
                "objects.contact.gender",
                "objects.contact.governmentAgency",
                "objects.contact.mandateReference",
                "objects.contact.name",
                "objects.contact.name2",
                "objects.contact.objectName",
                "objects.contact.sevClient",
                "objects.contact.status",
                "objects.contact.surename",
                "objects.contact.taxNumber",
                "objects.contact.taxOffice",
                "objects.contact.taxType",
                "objects.contact.titel",
                "objects.contact.update",
                "objects.contact.vatNumber",
                "objects.country.objectName",
                "objects.create",
                "objects.name",
                "objects.objectName",
                "objects.sevClient",
                "objects.update",
            ],
        ),
        (
            "contact_address_list",
            unread::<ContactAddress>("contact_address_list"),
            &[
                "objects.*.additionalInformation",
                "objects.*.category.objectName",
                "objects.*.contact.objectName",
                "objects.*.country.objectName",
                "objects.*.create",
                "objects.*.name",
                "objects.*.objectName",
                "objects.*.sevClient",
                "objects.*.update",
            ],
        ),
        (
            "credit_note",
            unread::<CreditNote>("credit_note"),
            &[
                "objects.*.accountEndDate",
                "objects.*.accountIntervall",
                "objects.*.accountLastCreditNote",
                "objects.*.accountNextCreditNote",
                "objects.*.additionalInformation",
                "objects.*.address",
                "objects.*.addressCountry.objectName",
                "objects.*.addressGender",
                "objects.*.addressName2",
                "objects.*.addressParentName",
                "objects.*.addressParentName2",
                "objects.*.bookingCategory",
                "objects.*.contact",
                "objects.*.contactPerson",
                "objects.*.create",
                "objects.*.createUser",
                "objects.*.creditNoteType",
                "objects.*.deliveryDate",
                "objects.*.deliveryDateUntil",
                "objects.*.discount",
                "objects.*.discountTime",
                "objects.*.footText",
                "objects.*.headText",
                "objects.*.header",
                "objects.*.isTransferred",
                "objects.*.objectName",
                "objects.*.origin.objectName",
                "objects.*.payDate",
                "objects.*.refSrcInvoice.objectName",
                "objects.*.reminderCharge",
                "objects.*.reminderDeadline",
                "objects.*.reminderDebit",
                "objects.*.reminderTotal",
                "objects.*.sendDate",
                "objects.*.sendType",
                "objects.*.sevClient",
                "objects.*.showNet",
                "objects.*.smallSettlement",
                "objects.*.sumDiscountGross",
                "objects.*.sumDiscountGrossForeignCurrency",
                "objects.*.sumDiscountNet",
                "objects.*.sumDiscountNetForeignCurrency",
                "objects.*.sumDiscounts",
                "objects.*.sumDiscountsForeignCurrency",
                "objects.*.sumGrossAccounting",
                "objects.*.sumGrossForeignCurrency",
                "objects.*.sumNetAccounting",
                "objects.*.sumNetForeignCurrency",
                "objects.*.sumTax",
                "objects.*.sumTaxAccounting",
                "objects.*.sumTaxForeignCurrency",
                "objects.*.taxNumber",
                "objects.*.taxRate",
                "objects.*.taxRule",
                "objects.*.taxText",
                "objects.*.taxType",
                "objects.*.timeToPay",
                "objects.*.update",
                "objects.*.vatNumber",
            ],
        ),
        (
            "invoice_draft",
            unread::<Invoice>("invoice_draft"),
            &[
                "objects.*.addressCountry.objectName",
                "objects.*.contact.objectName",
                "objects.*.contactPerson.objectName",
                "objects.*.createUser.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "invoice_enshrined",
            unread::<Invoice>("invoice_enshrined"),
            &[
                "objects.*.addressCountry.objectName",
                "objects.*.contact.objectName",
                "objects.*.contactPerson.objectName",
                "objects.*.createUser.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "invoice_list",
            unread::<Invoice>("invoice_list"),
            &[
                "objects.*.addressCountry.objectName",
                "objects.*.contact.objectName",
                "objects.*.contactPerson.objectName",
                "objects.*.createUser.objectName",
                "objects.*.objectName",
                "objects.*.origin.objectName",
                "objects.*.paymentMethod.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "payment_method_list",
            unread::<PaymentMethod>("payment_method_list"),
            &[
                "objects.*.additionalInformation",
                "objects.*.create",
                "objects.*.objectName",
                "objects.*.text",
                "objects.*.translationCode",
                "objects.*.update",
            ],
        ),
        (
            "receipt_guidance",
            unread::<ReceiptGuidance>("receipt_guidance"),
            &[
                "objects.*.allowedReceiptTypes",
                "objects.*.allowedTaxRules.*.description",
                "objects.*.description",
            ],
        ),
        (
            "transaction",
            unread::<Transaction>("transaction"),
            &[
                "objects.*.additionalInformation",
                "objects.*.checkAccount.objectName",
                "objects.*.compareHash",
                "objects.*.create",
                "objects.*.enshrined",
                "objects.*.entryText",
                "objects.*.externalId",
                "objects.*.feeAmount",
                "objects.*.gvCode",
                "objects.*.objectName",
                "objects.*.obonoReceiptUuid",
                "objects.*.payeePayerAcctNo",
                "objects.*.payeePayerBankCode",
                "objects.*.primaNotaNo",
                "objects.*.restoredAt",
                "objects.*.score",
                "objects.*.sevClient",
                "objects.*.update",
            ],
        ),
        (
            "transaction_deleted",
            unread::<Transaction>("transaction_deleted"),
            &[
                "objects.*.additionalInformation",
                "objects.*.checkAccount.objectName",
                "objects.*.compareHash",
                "objects.*.create",
                "objects.*.enshrined",
                "objects.*.entryText",
                "objects.*.externalId",
                "objects.*.feeAmount",
                "objects.*.gvCode",
                "objects.*.objectName",
                "objects.*.obonoReceiptUuid",
                "objects.*.payeePayerAcctNo",
                "objects.*.payeePayerBankCode",
                "objects.*.primaNotaNo",
                "objects.*.restoredAt",
                "objects.*.score",
                "objects.*.sevClient",
                "objects.*.update",
            ],
        ),
        (
            "transaction_list",
            unread::<Transaction>("transaction_list"),
            &[
                "objects.*.additionalInformation",
                "objects.*.checkAccount.objectName",
                "objects.*.compareHash",
                "objects.*.create",
                "objects.*.enshrined",
                "objects.*.entryText",
                "objects.*.externalId",
                "objects.*.feeAmount",
                "objects.*.gvCode",
                "objects.*.objectName",
                "objects.*.obonoReceiptUuid",
                "objects.*.payeePayerAcctNo",
                "objects.*.payeePayerBankCode",
                "objects.*.primaNotaNo",
                "objects.*.restoredAt",
                "objects.*.score",
                "objects.*.sevClient",
                "objects.*.targetTransaction.objectName",
                "objects.*.update",
            ],
        ),
        (
            "transaction_log",
            unread::<TransactionLogEntry>("transaction_log"),
            &[
                "objects.*.additionalInformation",
                "objects.*.checkAccountTransaction.objectName",
                "objects.*.create",
                "objects.*.objectName",
                "objects.*.sevClient",
            ],
        ),
        (
            "voucher",
            unread::<Voucher>("voucher"),
            &[
                "objects.*.accountingSpecialCase",
                "objects.*.additionalInformation",
                "objects.*.create",
                "objects.*.createUser",
                "objects.*.document.objectName",
                "objects.*.enshrined",
                "objects.*.hidden",
                "objects.*.iban",
                "objects.*.inSource",
                "objects.*.mileageRate",
                "objects.*.objectName",
                "objects.*.payDate",
                "objects.*.paymentDeadline",
                "objects.*.recurringEndDate",
                "objects.*.recurringInterval",
                "objects.*.recurringIntervall",
                "objects.*.recurringLastVoucher",
                "objects.*.recurringNextVoucher",
                "objects.*.recurringStartDate",
                "objects.*.resultDisdar",
                "objects.*.selectedForPaymentFile",
                "objects.*.sendType",
                "objects.*.sevClient",
                "objects.*.showNet",
                "objects.*.sumDiscountGross",
                "objects.*.sumDiscountGrossForeignCurrency",
                "objects.*.sumDiscountNet",
                "objects.*.sumDiscountNetForeignCurrency",
                "objects.*.sumGrossAccounting",
                "objects.*.sumGrossForeignCurrency",
                "objects.*.sumNetAccounting",
                "objects.*.sumNetForeignCurrency",
                "objects.*.sumTax",
                "objects.*.sumTaxAccounting",
                "objects.*.sumTaxForeignCurrency",
                "objects.*.supplierNameAtSave",
                "objects.*.taxRule.objectName",
                "objects.*.taxType",
                "objects.*.taxmaroStockAccount",
                "objects.*.tip",
                "objects.*.update",
                "objects.*.vatNumber",
            ],
        ),
        (
            "voucher_list",
            unread::<Voucher>("voucher_list"),
            &[
                "objects.*.accountingSpecialCase",
                "objects.*.additionalInformation",
                "objects.*.create",
                "objects.*.createUser",
                "objects.*.enshrined",
                "objects.*.hidden",
                "objects.*.iban",
                "objects.*.inSource",
                "objects.*.mileageRate",
                "objects.*.objectName",
                "objects.*.payDate",
                "objects.*.paymentDeadline",
                "objects.*.recurringEndDate",
                "objects.*.recurringInterval",
                "objects.*.recurringIntervall",
                "objects.*.recurringLastVoucher",
                "objects.*.recurringNextVoucher",
                "objects.*.recurringStartDate",
                "objects.*.resultDisdar",
                "objects.*.selectedForPaymentFile",
                "objects.*.sendType",
                "objects.*.sevClient",
                "objects.*.showNet",
                "objects.*.sumDiscountGross",
                "objects.*.sumDiscountGrossForeignCurrency",
                "objects.*.sumDiscountNet",
                "objects.*.sumDiscountNetForeignCurrency",
                "objects.*.sumGrossAccounting",
                "objects.*.sumGrossForeignCurrency",
                "objects.*.sumNetAccounting",
                "objects.*.sumNetForeignCurrency",
                "objects.*.sumTax",
                "objects.*.sumTaxAccounting",
                "objects.*.sumTaxForeignCurrency",
                "objects.*.supplierNameAtSave",
                "objects.*.taxRule.objectName",
                "objects.*.taxType",
                "objects.*.taxmaroStockAccount",
                "objects.*.tip",
                "objects.*.update",
                "objects.*.vatNumber",
            ],
        ),
        (
            "voucher_positions",
            unread::<VoucherPosition>("voucher_positions"),
            &[
                "objects.*.accountDatev.objectName",
                "objects.*.additionalInformation",
                "objects.*.assetMemoValue",
                "objects.*.cateringTaxRate",
                "objects.*.cateringTip",
                "objects.*.create",
                "objects.*.createNextPart",
                "objects.*.discountedValue",
                "objects.*.isAsset",
                "objects.*.isGwg",
                "objects.*.isPercentage",
                "objects.*.objectName",
                "objects.*.sevClient",
                "objects.*.specialAccountingField1",
                "objects.*.specialAccountingField2",
                "objects.*.specialAccountingField3",
                "objects.*.specialAccountingField4",
                "objects.*.specialAccountingField5",
                "objects.*.sum",
                "objects.*.sumDiscountForeignCurrency",
                "objects.*.sumGrossAccounting",
                "objects.*.sumGrossForeignCurrency",
                "objects.*.sumNetAccounting",
                "objects.*.sumNetForeignCurrency",
                "objects.*.sumTax",
                "objects.*.sumTaxAccounting",
                "objects.*.sumTaxForeignCurrency",
                "objects.*.update",
                "objects.*.voucher.objectName",
            ],
        ),
        (
            "write_communication_way_create",
            unread::<CommunicationWay>("write_communication_way_create"),
            &[
                "objects.additionalInformation",
                "objects.contact.objectName",
                "objects.create",
                "objects.key.objectName",
                "objects.objectName",
                "objects.sevClient",
                "objects.update",
            ],
        ),
        (
            "write_contact_address_create",
            unread::<ContactAddress>("write_contact_address_create"),
            &[
                "objects.additionalInformation",
                "objects.category.objectName",
                "objects.contact.academicTitle",
                "objects.contact.additionalInformation",
                "objects.contact.bankAccount",
                "objects.contact.bankNumber",
                "objects.contact.birthday",
                "objects.contact.buyerReference",
                "objects.contact.category",
                "objects.contact.create",
                "objects.contact.customerNumber",
                "objects.contact.defaultCashbackPercent",
                "objects.contact.defaultCashbackTime",
                "objects.contact.defaultDiscountAmount",
                "objects.contact.defaultDiscountPercentage",
                "objects.contact.defaultShowVat",
                "objects.contact.defaultTimeToPay",
                "objects.contact.description",
                "objects.contact.exemptVat",
                "objects.contact.familyname",
                "objects.contact.gender",
                "objects.contact.governmentAgency",
                "objects.contact.mandateReference",
                "objects.contact.name",
                "objects.contact.name2",
                "objects.contact.objectName",
                "objects.contact.sevClient",
                "objects.contact.status",
                "objects.contact.surename",
                "objects.contact.taxNumber",
                "objects.contact.taxOffice",
                "objects.contact.taxType",
                "objects.contact.titel",
                "objects.contact.update",
                "objects.contact.vatNumber",
                "objects.country.objectName",
                "objects.create",
                "objects.name",
                "objects.objectName",
                "objects.sevClient",
                "objects.update",
            ],
        ),
        (
            "write_contact_create",
            unread::<Contact>("write_contact_create"),
            &[
                "objects.academicTitle",
                "objects.additionalInformation",
                "objects.bankAccount",
                "objects.bankNumber",
                "objects.birthday",
                "objects.category",
                "objects.create",
                "objects.defaultCashbackPercent",
                "objects.defaultCashbackTime",
                "objects.defaultDiscountAmount",
                "objects.defaultDiscountPercentage",
                "objects.defaultShowVat",
                "objects.defaultTimeToPay",
                "objects.description",
                "objects.exemptVat",
                "objects.gender",
                "objects.governmentAgency",
                "objects.mandateReference",
                "objects.name2",
                "objects.objectName",
                "objects.sevClient",
                "objects.status",
                "objects.taxNumber",
                "objects.taxOffice",
                "objects.taxType",
                "objects.titel",
                "objects.update",
            ],
        ),
        (
            "write_credit_note_save",
            unread::<SavedCreditNote>("write_credit_note_save"),
            &[
                "objects.creditNote.accountEndDate",
                "objects.creditNote.accountIntervall",
                "objects.creditNote.accountLastCreditNote",
                "objects.creditNote.accountNextCreditNote",
                "objects.creditNote.additionalInformation",
                "objects.creditNote.address",
                "objects.creditNote.addressCountry.objectName",
                "objects.creditNote.addressGender",
                "objects.creditNote.addressName2",
                "objects.creditNote.addressParentName",
                "objects.creditNote.addressParentName2",
                "objects.creditNote.bookingCategory",
                "objects.creditNote.contact",
                "objects.creditNote.contactPerson",
                "objects.creditNote.create",
                "objects.creditNote.createUser",
                "objects.creditNote.creditNoteType",
                "objects.creditNote.deliveryDate",
                "objects.creditNote.deliveryDateUntil",
                "objects.creditNote.discount",
                "objects.creditNote.discountTime",
                "objects.creditNote.footText",
                "objects.creditNote.headText",
                "objects.creditNote.header",
                "objects.creditNote.isTransferred",
                "objects.creditNote.objectName",
                "objects.creditNote.payDate",
                "objects.creditNote.paymentMethod",
                "objects.creditNote.propertyIsEInvoice",
                "objects.creditNote.refSrcInvoice.objectName",
                "objects.creditNote.reminderCharge",
                "objects.creditNote.reminderDeadline",
                "objects.creditNote.reminderDebit",
                "objects.creditNote.reminderTotal",
                "objects.creditNote.sendDate",
                "objects.creditNote.sendType",
                "objects.creditNote.sevClient",
                "objects.creditNote.showNet",
                "objects.creditNote.smallSettlement",
                "objects.creditNote.sumDiscountGross",
                "objects.creditNote.sumDiscountGrossForeignCurrency",
                "objects.creditNote.sumDiscountNet",
                "objects.creditNote.sumDiscountNetForeignCurrency",
                "objects.creditNote.sumDiscounts",
                "objects.creditNote.sumDiscountsForeignCurrency",
                "objects.creditNote.sumGrossAccounting",
                "objects.creditNote.sumGrossForeignCurrency",
                "objects.creditNote.sumNetAccounting",
                "objects.creditNote.sumNetForeignCurrency",
                "objects.creditNote.sumTax",
                "objects.creditNote.sumTaxAccounting",
                "objects.creditNote.sumTaxForeignCurrency",
                "objects.creditNote.taxNumber",
                "objects.creditNote.taxRate",
                "objects.creditNote.taxRule",
                "objects.creditNote.taxText",
                "objects.creditNote.taxType",
                "objects.creditNote.timeToPay",
                "objects.creditNote.update",
                "objects.creditNote.vatNumber",
                "objects.creditNotePos",
            ],
        ),
        (
            "write_invoice_cancel",
            unread::<Invoice>("write_invoice_cancel"),
            &[
                "objects.contact.<embedded object>",
                "objects.contactPerson.objectName",
                "objects.createUser.objectName",
                "objects.objectName",
                "objects.origin.objectName",
                "objects.sevClient.objectName",
                "objects.taxRule.<embedded object>",
            ],
        ),
        (
            "quantity_probe_save",
            unread::<SavedInvoice>("quantity_probe_save"),
            &[
                "objects.invoice.addressCountry.objectName",
                "objects.invoice.contact.objectName",
                "objects.invoice.contactPerson.objectName",
                "objects.invoice.createUser.objectName",
                "objects.invoice.objectName",
                "objects.invoice.paymentMethod.objectName",
                "objects.invoice.sevClient.objectName",
                "objects.invoice.taxRule.objectName",
                "objects.invoicePos.*.invoice.<embedded object>",
                "objects.invoicePos.*.objectName",
                "objects.invoicePos.*.unity.<embedded object>",
            ],
        ),
        (
            "quantity_probe_get_invoice_pos",
            unread::<InvoicePosition>("quantity_probe_get_invoice_pos"),
            &[
                "objects.*.invoice.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.unity.objectName",
            ],
        ),
        (
            "write_invoice_save",
            unread::<SavedInvoice>("write_invoice_save"),
            &[
                "objects.invoice.addressCountry.objectName",
                "objects.invoice.contact.objectName",
                "objects.invoice.contactPerson.objectName",
                "objects.invoice.createUser.objectName",
                "objects.invoice.objectName",
                "objects.invoice.paymentMethod.objectName",
                "objects.invoice.sevClient.objectName",
                "objects.invoice.taxRule.objectName",
                "objects.invoicePos.*.invoice.<embedded object>",
                "objects.invoicePos.*.objectName",
                "objects.invoicePos.*.unity.<embedded object>",
            ],
        ),
        (
            "write_transaction_create",
            unread::<Transaction>("write_transaction_create"),
            &[
                "objects.additionalInformation",
                "objects.checkAccount.objectName",
                "objects.compareHash",
                "objects.create",
                "objects.enshrined",
                "objects.entryText",
                "objects.externalId",
                "objects.feeAmount",
                "objects.gvCode",
                "objects.objectName",
                "objects.obonoReceiptUuid",
                "objects.payeePayerAcctNo",
                "objects.payeePayerBankCode",
                "objects.primaNotaNo",
                "objects.restoredAt",
                "objects.score",
                "objects.sevClient",
                "objects.update",
            ],
        ),
        (
            "write_transaction_update",
            unread::<Transaction>("write_transaction_update"),
            &[
                "objects.additionalInformation",
                "objects.checkAccount.objectName",
                "objects.compareHash",
                "objects.create",
                "objects.enshrined",
                "objects.entryText",
                "objects.externalId",
                "objects.feeAmount",
                "objects.gvCode",
                "objects.objectName",
                "objects.obonoReceiptUuid",
                "objects.payeePayerAcctNo",
                "objects.payeePayerBankCode",
                "objects.primaNotaNo",
                "objects.restoredAt",
                "objects.score",
                "objects.sevClient",
                "objects.update",
            ],
        ),
        (
            "write_upload_temp_file",
            unread::<UploadedFile>("write_upload_temp_file"),
            &[
                "objects.content",
                "objects.contentHash",
                "objects.isEInvoice",
            ],
        ),
        (
            "write_voucher_save_create",
            unread::<SavedVoucher>("write_voucher_save_create"),
            &[
                "objects.vatdrop",
                "objects.voucher.accountingSpecialCase",
                "objects.voucher.additionalInformation",
                "objects.voucher.create",
                "objects.voucher.createUser",
                "objects.voucher.enshrined",
                "objects.voucher.hidden",
                "objects.voucher.iban",
                "objects.voucher.inSource",
                "objects.voucher.mileageRate",
                "objects.voucher.objectName",
                "objects.voucher.payDate",
                "objects.voucher.paymentDeadline",
                "objects.voucher.propertyCreationOrigin",
                "objects.voucher.recurringEndDate",
                "objects.voucher.recurringInterval",
                "objects.voucher.recurringIntervall",
                "objects.voucher.recurringLastVoucher",
                "objects.voucher.recurringNextVoucher",
                "objects.voucher.recurringStartDate",
                "objects.voucher.resultDisdar",
                "objects.voucher.selectedForPaymentFile",
                "objects.voucher.sendType",
                "objects.voucher.sevClient",
                "objects.voucher.showNet",
                "objects.voucher.sumDiscountGross",
                "objects.voucher.sumDiscountGrossForeignCurrency",
                "objects.voucher.sumDiscountNet",
                "objects.voucher.sumDiscountNetForeignCurrency",
                "objects.voucher.sumGrossAccounting",
                "objects.voucher.sumGrossForeignCurrency",
                "objects.voucher.sumNetAccounting",
                "objects.voucher.sumNetForeignCurrency",
                "objects.voucher.sumTax",
                "objects.voucher.sumTaxAccounting",
                "objects.voucher.sumTaxForeignCurrency",
                "objects.voucher.supplierNameAtSave",
                "objects.voucher.taxRule.objectName",
                "objects.voucher.taxType",
                "objects.voucher.taxmaroStockAccount",
                "objects.voucher.tip",
                "objects.voucher.update",
                "objects.voucher.vatNumber",
                "objects.voucherPos.*.accountDatev.objectName",
                "objects.voucherPos.*.additionalInformation",
                "objects.voucherPos.*.assetMemoValue",
                "objects.voucherPos.*.cateringTaxRate",
                "objects.voucherPos.*.cateringTip",
                "objects.voucherPos.*.create",
                "objects.voucherPos.*.createNextPart",
                "objects.voucherPos.*.discountedValue",
                "objects.voucherPos.*.isAsset",
                "objects.voucherPos.*.isGwg",
                "objects.voucherPos.*.isPercentage",
                "objects.voucherPos.*.objectName",
                "objects.voucherPos.*.sevClient",
                "objects.voucherPos.*.specialAccountingField1",
                "objects.voucherPos.*.specialAccountingField2",
                "objects.voucherPos.*.specialAccountingField3",
                "objects.voucherPos.*.specialAccountingField4",
                "objects.voucherPos.*.specialAccountingField5",
                "objects.voucherPos.*.sum",
                "objects.voucherPos.*.sumDiscountForeignCurrency",
                "objects.voucherPos.*.sumGrossAccounting",
                "objects.voucherPos.*.sumGrossForeignCurrency",
                "objects.voucherPos.*.sumNetAccounting",
                "objects.voucherPos.*.sumNetForeignCurrency",
                "objects.voucherPos.*.sumTax",
                "objects.voucherPos.*.sumTaxAccounting",
                "objects.voucherPos.*.sumTaxForeignCurrency",
                "objects.voucherPos.*.update",
                "objects.voucherPos.*.voucher.objectName",
            ],
        ),
        (
            "write_voucher_save_update_release_attach",
            unread::<SavedVoucher>("write_voucher_save_update_release_attach"),
            &[
                "objects.document",
                "objects.vatdrop",
                "objects.voucher.accountingSpecialCase",
                "objects.voucher.additionalInformation",
                "objects.voucher.create",
                "objects.voucher.createUser",
                "objects.voucher.document.objectName",
                "objects.voucher.enshrined",
                "objects.voucher.hidden",
                "objects.voucher.iban",
                "objects.voucher.inSource",
                "objects.voucher.mileageRate",
                "objects.voucher.objectName",
                "objects.voucher.payDate",
                "objects.voucher.paymentDeadline",
                "objects.voucher.propertyIsBookkeepingV1",
                "objects.voucher.recurringEndDate",
                "objects.voucher.recurringInterval",
                "objects.voucher.recurringIntervall",
                "objects.voucher.recurringLastVoucher",
                "objects.voucher.recurringNextVoucher",
                "objects.voucher.recurringStartDate",
                "objects.voucher.resultDisdar",
                "objects.voucher.selectedForPaymentFile",
                "objects.voucher.sendType",
                "objects.voucher.sevClient",
                "objects.voucher.showNet",
                "objects.voucher.sumDiscountGross",
                "objects.voucher.sumDiscountGrossForeignCurrency",
                "objects.voucher.sumDiscountNet",
                "objects.voucher.sumDiscountNetForeignCurrency",
                "objects.voucher.sumGrossAccounting",
                "objects.voucher.sumGrossForeignCurrency",
                "objects.voucher.sumNetAccounting",
                "objects.voucher.sumNetForeignCurrency",
                "objects.voucher.sumTax",
                "objects.voucher.sumTaxAccounting",
                "objects.voucher.sumTaxForeignCurrency",
                "objects.voucher.supplierNameAtSave",
                "objects.voucher.taxRule.objectName",
                "objects.voucher.taxType",
                "objects.voucher.taxmaroStockAccount",
                "objects.voucher.tip",
                "objects.voucher.update",
                "objects.voucher.vatNumber",
                "objects.voucherPos.*.accountDatev.objectName",
                "objects.voucherPos.*.additionalInformation",
                "objects.voucherPos.*.assetMemoValue",
                "objects.voucherPos.*.cateringTaxRate",
                "objects.voucherPos.*.cateringTip",
                "objects.voucherPos.*.create",
                "objects.voucherPos.*.createNextPart",
                "objects.voucherPos.*.discountedValue",
                "objects.voucherPos.*.isAsset",
                "objects.voucherPos.*.isGwg",
                "objects.voucherPos.*.isPercentage",
                "objects.voucherPos.*.objectName",
                "objects.voucherPos.*.sevClient",
                "objects.voucherPos.*.specialAccountingField1",
                "objects.voucherPos.*.specialAccountingField2",
                "objects.voucherPos.*.specialAccountingField3",
                "objects.voucherPos.*.specialAccountingField4",
                "objects.voucherPos.*.specialAccountingField5",
                "objects.voucherPos.*.sum",
                "objects.voucherPos.*.sumDiscountForeignCurrency",
                "objects.voucherPos.*.sumGrossAccounting",
                "objects.voucherPos.*.sumGrossForeignCurrency",
                "objects.voucherPos.*.sumNetAccounting",
                "objects.voucherPos.*.sumNetForeignCurrency",
                "objects.voucherPos.*.sumTax",
                "objects.voucherPos.*.sumTaxAccounting",
                "objects.voucherPos.*.sumTaxForeignCurrency",
                "objects.voucherPos.*.update",
                "objects.voucherPos.*.voucher.objectName",
            ],
        ),
    ]
}

/// Fixtures that are deliberately not decoded here, with the reason.
const SKIPPED: &[(&str, &str)] = &[
    ("balance_at_date", "a bare amount string, no object"),
    (
        "bookkeeping_system_version",
        "read by a private one-field struct",
    ),
    ("error_404_code_151", "an error body"),
    ("error_422_offline_account", "an error body"),
    ("invoice_pdf", "read into a private wire type (DocumentPdf)"),
    ("invoice_xml", "a bare XML string, no object"),
    (
        "write_transaction_delete",
        "answer is a list holding null, no read type",
    ),
    (
        "write_voucher_book_amount",
        "write answer the client discards, no read type",
    ),
    (
        "write_voucher_reset_to_open",
        "write answer the client discards, no read type",
    ),
];

#[test]
fn unread_keys_equal_the_named_lists() {
    let mut failures = Vec::new();
    for (name, found, listed) in covered() {
        let mut expected: Vec<String> = listed.iter().map(|k| (*k).to_owned()).collect();
        // A form-unmeasured key is expected exactly where the fixture has it.
        expected.extend(
            form_unmeasured(name)
                .into_iter()
                .map(|(key, _)| key)
                .filter(|key| found.contains(key)),
        );
        expected.sort();
        if found != expected {
            failures.push(format!(
                "{name}: unread keys changed ({} -> {}); new list:\n{found:#?}",
                expected.len(),
                found.len()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A key on a form-unmeasured list must still be unread: when a field is added
/// for it, it leaves the list in the same commit.
#[test]
fn form_unmeasured_keys_are_still_unread_in_some_fixture() {
    let all = covered();
    let bare = |path: &str| path.rsplit('.').next().unwrap_or(path).to_owned();
    let mut stale = BTreeSet::new();
    for (name, _, _) in &all {
        for (key, recorded) in form_unmeasured(name) {
            let shown = all.iter().any(|(other, found, _)| {
                form_unmeasured(other)
                    .iter()
                    .any(|(k, _)| bare(k) == bare(&key))
                    && found.iter().any(|f| bare(f) == bare(&key))
            });
            if recorded && !shown {
                stale.insert(bare(&key));
            }
        }
    }
    assert!(stale.is_empty(), "no fixture shows these unread: {stale:?}");
}

#[test]
fn every_fixture_is_covered_or_skipped() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let on_disk: BTreeSet<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".json").map(str::to_owned))
        .collect();
    let known: BTreeSet<String> = covered()
        .into_iter()
        .map(|(n, ..)| n.to_owned())
        .chain(SKIPPED.iter().map(|(n, _)| (*n).to_owned()))
        .collect();
    assert_eq!(
        on_disk, known,
        "fixtures and the covered/skipped lists differ"
    );
}
