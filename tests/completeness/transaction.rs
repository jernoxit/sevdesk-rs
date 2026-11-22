//! Completeness lists of the transaction resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

/// Transaction keys that no recording or survey shows filled and that the
/// OpenAPI spec does not type (form unmeasured): not read until a measurement
/// shows them. The flag says whether any fixture carries the key at all.
const TRANSACTION_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 172 survey objects
    ("compareHash", true),           // not in the spec; null in all 172 survey objects
    ("externalId", true),            // not in the spec; null in all 172 survey objects
    ("feeAmount", true),             // not in the spec; null in all 172 survey objects
    ("obonoReceiptUuid", true),      // not in the spec; null in all 172 survey objects
    ("restoredAt", true),            // not in the spec; null in all 172 survey objects
    ("score", true),                 // not in the spec; null in all 172 survey objects
];

/// CheckAccountTransactionLog keys of the same kind (the spec has no schema).
const TRANSACTION_LOG_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 136 survey objects
];

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "transaction",
            unread::<Transaction>("transaction"),
            &[
                "objects.*.checkAccount.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "transaction_deleted",
            unread::<Transaction>("transaction_deleted"),
            &[
                "objects.*.checkAccount.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "transaction_list",
            unread::<Transaction>("transaction_list"),
            &[
                "objects.*.checkAccount.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.targetTransaction.objectName",
            ],
        ),
        (
            "sample_transaction_list",
            unread::<Transaction>("sample_transaction_list"),
            &[
                "objects.*.checkAccount.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.sourceTransaction.objectName",
                "objects.*.targetTransaction.objectName",
            ],
        ),
        (
            "write_transaction_create",
            unread::<Transaction>("write_transaction_create"),
            &[
                "objects.checkAccount.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "write_transaction_update",
            unread::<Transaction>("write_transaction_update"),
            &[
                "objects.checkAccount.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "transaction_log",
            unread::<TransactionLogEntry>("transaction_log"),
            &[
                "objects.*.checkAccountTransaction.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "sample_transaction_log_list",
            unread::<TransactionLogEntry>("sample_transaction_log_list"),
            &[
                "objects.*.checkAccountTransaction.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "transaction" | "transaction_deleted" | "transaction_list" | "sample_transaction_list" => {
            under("objects.*.", TRANSACTION_FORM_UNMEASURED)
        }
        "write_transaction_create" | "write_transaction_update" => {
            under("objects.", TRANSACTION_FORM_UNMEASURED)
        }
        "transaction_log" | "sample_transaction_log_list" => {
            under("objects.*.", TRANSACTION_LOG_FORM_UNMEASURED)
        }
        _ => return None,
    })
}
