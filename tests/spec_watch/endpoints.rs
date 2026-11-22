//! Check 4: every `METHOD path` of the spec is either implemented by a crate
//! method or knowingly not.

use super::exact;
use serde_yaml_ng::Value;
use std::collections::BTreeSet;

/// Spec endpoints a crate method calls, with the method's name. Not in the
/// spec, but called anyway (measured): `DELETE /Invoice/{id}`,
/// `DELETE /Voucher/{id}`, `GET /CreditNote/{id}/getXml`, `/SevUser`,
/// `/PaymentMethod`, `/CheckAccountTransactionLog`, `/SevSequence/Factory/getByType`.
pub(super) const IMPLEMENTED: &[(&str, &str)] = &[
    (
        "DELETE /CheckAccountTransaction/{checkAccountTransactionId}",
        "delete_transaction",
    ),
    ("DELETE /CreditNote/{creditNoteId}", "delete_credit_note"),
    ("GET /CheckAccount", "list_check_accounts"),
    ("GET /CheckAccount/{checkAccountId}", "check_account"),
    (
        "GET /CheckAccount/{checkAccountId}/getBalanceAtDate",
        "balance_at_date",
    ),
    ("GET /CheckAccountTransaction", "list_transactions"),
    (
        "GET /CheckAccountTransaction/{checkAccountTransactionId}",
        "transaction",
    ),
    ("GET /CommunicationWay", "communication_ways"),
    (
        "GET /Contact",
        "list_contacts_by_name, find_contact_by_customer_number",
    ),
    ("GET /Contact/{contactId}", "contact"),
    ("GET /ContactAddress", "contact_addresses"),
    ("GET /CreditNote", "list_credit_notes"),
    ("GET /CreditNote/{creditNoteId}", "credit_note"),
    ("GET /CreditNote/{creditNoteId}/getPdf", "credit_note_pdf"),
    ("GET /Invoice", "list_invoices"),
    ("GET /Invoice/{invoiceId}", "invoice"),
    ("GET /Invoice/{invoiceId}/getPdf", "invoice_pdf"),
    ("GET /Invoice/{invoiceId}/getXml", "invoice_xml"),
    (
        "GET /ReceiptGuidance/forExpense",
        "receipt_guidance_for_expense",
    ),
    (
        "GET /Tools/bookkeepingSystemVersion",
        "bookkeeping_system_version",
    ),
    ("GET /Voucher", "list_vouchers"),
    ("GET /Voucher/{voucherId}", "voucher"),
    ("GET /VoucherPos", "voucher_positions"),
    (
        "POST /CheckAccount/Factory/fileImportAccount",
        "create_file_import_account",
    ),
    ("POST /CheckAccountTransaction", "create_transaction"),
    ("POST /CommunicationWay", "create_communication_way"),
    ("POST /Contact", "create_contact"),
    ("POST /ContactAddress", "create_contact_address"),
    (
        "POST /CreditNote/Factory/saveCreditNote",
        "save_credit_note",
    ),
    (
        "POST /CreditNote/{creditNoteId}/sendViaEmail",
        "send_credit_note_via_email",
    ),
    ("POST /Invoice/Factory/saveInvoice", "save_invoice"),
    ("POST /Invoice/{invoiceId}/cancelInvoice", "cancel_invoice"),
    (
        "POST /Invoice/{invoiceId}/sendViaEmail",
        "send_invoice_via_email",
    ),
    ("POST /Voucher/Factory/saveVoucher", "save_voucher"),
    ("POST /Voucher/Factory/uploadTempFile", "upload_temp_file"),
    ("PUT /CheckAccount/{checkAccountId}", "update_check_account"),
    (
        "PUT /CheckAccountTransaction/{checkAccountTransactionId}",
        "update_transaction",
    ),
    (
        "PUT /CheckAccountTransaction/{checkAccountTransactionId}/enshrine",
        "enshrine_transaction",
    ),
    (
        "PUT /CommunicationWay/{communicationWayId}",
        "update_communication_way",
    ),
    ("PUT /Contact/{contactId}", "update_contact"),
    (
        "PUT /ContactAddress/{contactAddressId}",
        "update_contact_address",
    ),
    (
        "PUT /CreditNote/{creditNoteId}/bookAmount",
        "book_credit_note_amount",
    ),
    (
        "PUT /CreditNote/{creditNoteId}/changeParameter",
        "change_credit_note_parameter",
    ),
    (
        "PUT /CreditNote/{creditNoteId}/enshrine",
        "enshrine_credit_note",
    ),
    (
        "PUT /CreditNote/{creditNoteId}/resetToOpen",
        "reset_credit_note_to_open",
    ),
    (
        "PUT /CreditNote/{creditNoteId}/sendBy",
        "send_credit_note_by",
    ),
    ("PUT /Invoice/{invoiceId}/bookAmount", "book_invoice_amount"),
    (
        "PUT /Invoice/{invoiceId}/changeParameter",
        "change_invoice_parameter",
    ),
    ("PUT /Invoice/{invoiceId}/enshrine", "enshrine_invoice"),
    (
        "PUT /Invoice/{invoiceId}/resetToOpen",
        "reset_invoice_to_open",
    ),
    ("PUT /Invoice/{invoiceId}/sendBy", "send_invoice_by"),
    ("PUT /Voucher/{voucherId}/bookAmount", "book_voucher_amount"),
    (
        "PUT /Voucher/{voucherId}/resetToDraft",
        "reset_voucher_to_draft",
    ),
    (
        "PUT /Voucher/{voucherId}/resetToOpen",
        "reset_voucher_to_open",
    ),
];

/// Spec endpoints no crate method calls.
pub(super) const NOT_IMPLEMENTED: &[&str] = &[
    "DELETE /AccountingContact/{accountingContactId}",
    "DELETE /CheckAccount/{checkAccountId}",
    "DELETE /CommunicationWay/{communicationWayId}",
    "DELETE /Contact/{contactId}",
    "DELETE /ContactAddress/{contactAddressId}",
    "DELETE /ContactCustomField/{contactCustomFieldId}",
    "DELETE /ContactCustomFieldSetting/{contactCustomFieldSettingId}",
    "DELETE /Order/{orderId}",
    "DELETE /OrderPos/{orderPosId}",
    "DELETE /PrivateTransactionRule/{id}",
    "DELETE /Tag/{tagId}",
    "GET /AccountingContact",
    "GET /AccountingContact/{accountingContactId}",
    "GET /CommunicationWay/{communicationWayId}",
    "GET /CommunicationWayKey",
    "GET /Contact/Factory/findContactsByCustomFieldValue",
    "GET /Contact/Factory/getNextCustomerNumber",
    "GET /Contact/Mapper/checkCustomerNumberAvailability",
    "GET /Contact/{contactId}/getTabsItemCount",
    "GET /ContactAddress/{contactAddressId}",
    "GET /ContactCustomField",
    "GET /ContactCustomField/{contactCustomFieldId}",
    "GET /ContactCustomFieldSetting",
    "GET /ContactCustomFieldSetting/{contactCustomFieldSettingId}",
    "GET /ContactCustomFieldSetting/{contactCustomFieldSettingId}/getReferenceCount",
    "GET /CreditNote/{creditNoteId}/sendByWithRender",
    "GET /CreditNotePos",
    "GET /DocServer/getLetterpapersWithThumb",
    "GET /DocServer/getTemplatesWithThumb",
    "GET /Export/contactListCsv",
    "GET /Export/createDatevCsvZipExportJob",
    "GET /Export/createDatevXmlZipExportJob",
    "GET /Export/creditNoteCsv",
    "GET /Export/datevCSV",
    "GET /Export/invoiceCsv",
    "GET /Export/invoiceZip",
    "GET /Export/transactionsCsv",
    "GET /Export/voucherListCsv",
    "GET /Export/voucherZip",
    "GET /ExportJob/jobDownloadInfo",
    "GET /Invoice/{invoiceId}/getIsPartiallyPaid",
    "GET /Invoice/{invoiceId}/getPositions",
    "GET /InvoicePos",
    "GET /Order",
    "GET /Order/{orderId}",
    "GET /Order/{orderId}/getDiscounts",
    "GET /Order/{orderId}/getPdf",
    "GET /Order/{orderId}/getPositions",
    "GET /Order/{orderId}/getRelatedObjects",
    "GET /OrderPos",
    "GET /OrderPos/{orderPosId}",
    "GET /Part",
    "GET /Part/{partId}",
    "GET /Part/{partId}/getStock",
    "GET /PrivateTransactionRule",
    "GET /Progress/generateDownloadHash",
    "GET /Progress/getProgress",
    "GET /ReceiptGuidance/forAccountNumber",
    "GET /ReceiptGuidance/forAllAccounts",
    "GET /ReceiptGuidance/forRevenue",
    "GET /ReceiptGuidance/forTaxRule",
    "GET /Report/contactlist",
    "GET /Report/invoicelist",
    "GET /Report/orderlist",
    "GET /Report/voucherlist",
    "GET /Tag",
    "GET /Tag/{tagId}",
    "GET /TagRelation",
    "GET /Textparser/fetchDictionaryEntriesByType",
    "POST /AccountingContact",
    "POST /CheckAccount/Factory/clearingAccount",
    "POST /ContactCustomField",
    "POST /ContactCustomFieldSetting",
    "POST /CreditNote/Factory/createFromInvoice",
    "POST /CreditNote/Factory/createFromVoucher",
    "POST /Invoice/Factory/createInvoiceFromOrder",
    "POST /Invoice/Factory/createInvoiceReminder",
    "POST /Invoice/{invoiceId}/render",
    "POST /Order/Factory/createContractNoteFromOrder",
    "POST /Order/Factory/createPackingListFromOrder",
    "POST /Order/Factory/saveOrder",
    "POST /Order/{orderId}/sendViaEmail",
    "POST /Part",
    "POST /PrivateTransactionRule",
    "POST /Tag/Factory/create",
    "PUT /AccountingContact/{accountingContactId}",
    "PUT /ContactCustomField/{contactCustomFieldId}",
    "PUT /ContactCustomFieldSetting/{contactCustomFieldSettingId}",
    "PUT /CreditNote/{creditNoteId}",
    "PUT /CreditNote/{creditNoteId}/resetToDraft",
    "PUT /Invoice/{invoiceId}/resetToDraft",
    "PUT /Order/{orderId}",
    "PUT /Order/{orderId}/changeParameter",
    "PUT /Order/{orderId}/sendBy",
    "PUT /OrderPos/{orderPosId}",
    "PUT /Part/{partId}",
    "PUT /SevClient/{SevClientId}/updateExportConfig",
    "PUT /Tag/{tagId}",
    "PUT /Voucher/{voucherId}",
    "PUT /Voucher/{voucherId}/enshrine",
];

pub(super) fn problems(spec: &Value) -> Vec<String> {
    let mut found = BTreeSet::new();
    let paths = spec
        .get("paths")
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("spec has no paths"));
    for (path, item) in paths {
        let (Some(path), Some(item)) = (path.as_str(), item.as_mapping()) else {
            continue;
        };
        for method in item.keys().filter_map(Value::as_str) {
            if matches!(method, "get" | "post" | "put" | "delete" | "patch") {
                found.insert(format!("{} {path}", method.to_uppercase()));
            }
        }
    }
    let listed: Vec<(&str, &str)> = IMPLEMENTED
        .iter()
        .copied()
        .chain(NOT_IMPLEMENTED.iter().map(|e| (*e, "")))
        .collect();
    let mut out: Vec<String> = exact("SPEC_ENDPOINTS", &found, &listed)
        .into_iter()
        .collect();
    let both: Vec<&str> = IMPLEMENTED
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| NOT_IMPLEMENTED.contains(e))
        .collect();
    if !both.is_empty() {
        out.push(format!(
            "endpoints in both IMPLEMENTED and NOT_IMPLEMENTED: {both:?}"
        ));
    }
    out
}
