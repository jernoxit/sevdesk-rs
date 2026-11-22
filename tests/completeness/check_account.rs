//! Completeness lists of the check account resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

/// CheckAccount keys that no recording or survey shows filled and that the
/// OpenAPI spec does not type (form unmeasured): not read until a measurement
/// shows them. The flag says whether any fixture carries the key at all.
const CHECK_ACCOUNT_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 11 survey objects
    ("checkAccId", true),            // not in the spec; null in all 11 survey objects
    ("countryCode", true),           // not in the spec; null in all 11 survey objects
    ("pin", true),                   // not in the spec; null in all 11 survey objects
    ("translationCode", true),       // not in the spec; null in all 11 survey objects
];

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "check_account",
            unread::<CheckAccount>("check_account"),
            &["objects.*.objectName", "objects.*.sevClient.objectName"],
        ),
        (
            "check_account_list",
            unread::<CheckAccount>("check_account_list"),
            &["objects.*.objectName", "objects.*.sevClient.objectName"],
        ),
        (
            "sample_check_account_list",
            unread::<CheckAccount>("sample_check_account_list"),
            &["objects.*.objectName", "objects.*.sevClient.objectName"],
        ),
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "check_account" | "check_account_list" | "sample_check_account_list" => {
            under("objects.*.", CHECK_ACCOUNT_FORM_UNMEASURED)
        }
        _ => return None,
    })
}
