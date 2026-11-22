//! Completeness lists of the misc resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

/// PaymentMethod keys that no measurement shows filled and that the OpenAPI
/// spec does not type (`/PaymentMethod` is not in it): not read until a
/// measurement shows them. The flag says whether any fixture carries the key.
const PAYMENT_METHOD_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 6 + 6 recorded objects
];

/// SevSequence keys with no measured form and no spec (`/SevSequence` is not
/// in it).
const SEV_SEQUENCE_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in both survey objects
];

/// UploadedFile keys with no measured form: `content` is `[]` in the only
/// recording, and the spec's array items carry only `example: null`.
const UPLOADED_FILE_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("content", true), // spec: array of untyped items; `[]` in the only recording
];

/// SevUser keys the shape survey found always `null`; `/SevUser` is not in
/// the spec. The fixture is SYNTHETIC (built from the survey's forms, no live
/// values), so it carries these as `null` by construction.
const SEV_USER_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in the survey object
    ("endDate", true),               // not in the spec; null in the survey object
    ("smtpHost", true),              // not in the spec; null in the survey object
    ("smtpMail", true),              // not in the spec; null in the survey object
    ("smtpName", true),              // not in the spec; null in the survey object
    ("smtpPort", true),              // not in the spec; null in the survey object
    ("smtpSsl", true),               // not in the spec; null in the survey object
    ("smtpUser", true),              // not in the spec; null in the survey object
    ("startDate", true),             // not in the spec; null in the survey object
];

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "payment_method_list",
            unread::<PaymentMethod>("payment_method_list"),
            &["objects.*.objectName"],
        ),
        (
            "receipt_guidance",
            unread::<ReceiptGuidance>("receipt_guidance"),
            &[],
        ),
        (
            "sample_payment_method_list",
            unread::<PaymentMethod>("sample_payment_method_list"),
            &["objects.*.objectName"],
        ),
        (
            "sample_sev_sequence_list",
            unread::<SevSequence>("sample_sev_sequence_list"),
            &["objects.*.objectName", "objects.*.sevClient.objectName"],
        ),
        // Synthetic: built from the shape survey's forms, no live values.
        (
            "sample_sev_user_list",
            unread::<SevUser>("sample_sev_user_list"),
            &["objects.*.objectName", "objects.*.sevClient.objectName"],
        ),
        (
            "write_upload_temp_file",
            unread::<UploadedFile>("write_upload_temp_file"),
            &[],
        ),
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "payment_method_list" | "sample_payment_method_list" => {
            under("objects.*.", PAYMENT_METHOD_FORM_UNMEASURED)
        }
        "sample_sev_sequence_list" => under("objects.*.", SEV_SEQUENCE_FORM_UNMEASURED),
        "sample_sev_user_list" => under("objects.*.", SEV_USER_FORM_UNMEASURED),
        "write_upload_temp_file" => under("objects.", UPLOADED_FILE_FORM_UNMEASURED),
        _ => return None,
    })
}
