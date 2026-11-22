//! Completeness lists of the check account resource group.

use super::{Covered, unread};
use sevdesk::*;

pub(super) fn covered() -> Vec<Covered> {
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
    ]
}
