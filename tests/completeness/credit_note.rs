//! Completeness lists of the credit note resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

/// CreditNote keys of the same kind (all `null` in the 196 surveyed notes).
const CREDIT_NOTE_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("accountEndDate", true), // not in the spec; null in all 196 survey objects
    ("accountIntervall", true), // not in the spec; null in all 196 survey objects
    ("accountLastCreditNote", true), // not in the spec; null in all 196 survey objects
    ("accountNextCreditNote", true), // not in the spec; null in all 196 survey objects
    ("additionalInformation", true), // not in the spec; null in all 196 survey objects
    ("addressGender", true),  // not in the spec; null in all 196 survey objects
    ("addressName2", true),   // not in the spec; null in all 196 survey objects
    ("addressParentName", true), // not in the spec; null in all 196 survey objects
    ("addressParentName2", true), // not in the spec; null in all 196 survey objects
    ("reminderCharge", true), // not in the response spec; null in all 196 survey objects
    ("reminderDeadline", true), // not in the response spec; null in all 196 survey objects
    ("reminderDebit", true),  // not in the response spec; null in all 196 survey objects
    ("reminderTotal", true),  // not in the response spec; null in all 196 survey objects
    ("taxNumber", true),      // not in the spec; null in all 196 survey objects
    ("timeToPay", true),      // not in the spec; null in all 196 survey objects
    ("vatNumber", true),      // not in the spec; null in all 196 survey objects
];

/// CreditNoteLog keys of the same kind.
const CREDIT_NOTE_LOG_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in the recording
];

/// CreditNotePos keys of the same kind (all `null` in the 206 surveyed positions).
const CREDIT_NOTE_POSITION_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 206 survey objects
    ("createNextPart", true),        // not in the spec; null in all 206 survey objects
];

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "credit_note",
            unread::<CreditNote>("credit_note"),
            &[
                "objects.*.addressCountry.objectName",
                "objects.*.contact.objectName",
                "objects.*.contactPerson.objectName",
                "objects.*.createUser.objectName",
                "objects.*.objectName",
                "objects.*.origin.objectName",
                "objects.*.refSrcInvoice.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "answer_credit_note_sendBy",
            unread::<CreditNote>("answer_credit_note_sendBy"),
            &[
                "objects.addressCountry.objectName",
                "objects.contact.objectName",
                "objects.contactPerson.objectName",
                "objects.createUser.objectName",
                "objects.objectName",
                "objects.paymentMethod.objectName",
                "objects.refSrcInvoice.objectName",
                "objects.sevClient.objectName",
                "objects.taxRule.objectName",
            ],
        ),
        (
            "answer_credit_note_resetToOpen",
            unread::<CreditNote>("answer_credit_note_resetToOpen"),
            &[
                "objects.addressCountry.objectName",
                "objects.contact.objectName",
                "objects.contactPerson.objectName",
                "objects.createUser.objectName",
                "objects.objectName",
                "objects.paymentMethod.objectName",
                "objects.refSrcInvoice.objectName",
                "objects.sevClient.objectName",
                "objects.taxRule.objectName",
            ],
        ),
        (
            "answer_credit_note_bookAmount",
            unread::<CreditNoteLog>("answer_credit_note_bookAmount"),
            &[
                "objects.creditNote.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "sample_credit_note_list",
            unread::<CreditNote>("sample_credit_note_list"),
            &[
                "objects.*.accountingType.objectName",
                "objects.*.addressCountry.objectName",
                "objects.*.contact.objectName",
                "objects.*.contactPerson.objectName",
                "objects.*.createUser.objectName",
                "objects.*.origin.objectName",
                "objects.*.paymentMethod.objectName",
                "objects.*.refSrcInvoice.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
                "objects.*.objectName",
            ],
        ),
        (
            "sample_credit_note_pos_list",
            unread::<CreditNotePosition>("sample_credit_note_pos_list"),
            &[
                "objects.*.creditNote.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.unity.objectName",
            ],
        ),
        (
            "write_credit_note_save",
            unread::<SavedCreditNote>("write_credit_note_save"),
            &[
                "objects.creditNote.addressCountry.objectName",
                "objects.creditNote.contact.objectName",
                "objects.creditNote.contactPerson.objectName",
                "objects.creditNote.createUser.objectName",
                "objects.creditNote.objectName",
                "objects.creditNote.paymentMethod.objectName",
                "objects.creditNote.refSrcInvoice.objectName",
                "objects.creditNote.sevClient.objectName",
                "objects.creditNote.taxRule.objectName",
                "objects.creditNotePos.*.creditNote.objectName",
                "objects.creditNotePos.*.objectName",
                "objects.creditNotePos.*.sevClient.objectName",
                "objects.creditNotePos.*.unity.objectName",
            ],
        ),
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "credit_note" | "sample_credit_note_list" => {
            under("objects.*.", CREDIT_NOTE_FORM_UNMEASURED)
        }
        "answer_credit_note_sendBy" | "answer_credit_note_resetToOpen" => {
            under("objects.", CREDIT_NOTE_FORM_UNMEASURED)
        }
        "answer_credit_note_bookAmount" => under("objects.", CREDIT_NOTE_LOG_FORM_UNMEASURED),
        "sample_credit_note_pos_list" => under("objects.*.", CREDIT_NOTE_POSITION_FORM_UNMEASURED),
        "write_credit_note_save" => {
            let mut keys = under("objects.creditNote.", CREDIT_NOTE_FORM_UNMEASURED);
            keys.extend(under(
                "objects.creditNotePos.*.",
                CREDIT_NOTE_POSITION_FORM_UNMEASURED,
            ));
            keys
        }
        _ => return None,
    })
}
