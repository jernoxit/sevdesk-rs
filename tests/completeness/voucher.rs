//! Completeness lists of the voucher resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

/// Voucher keys that no recording or survey shows filled and that the OpenAPI
/// spec does not type (form unmeasured): not read until a measurement shows
/// them. The flag says whether any fixture carries the key at all.
const VOUCHER_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("accountingSpecialCase", true), // not in the spec; null in all 16 survey objects
    ("additionalInformation", true), // not in the spec; null in all 16 survey objects
    ("iban", true),                  // not in the spec; null in all 16 survey objects
    ("inSource", true),              // not in the spec; null in all 16 survey objects
    ("propertyCreationOrigin", true), // not in the spec; null in the one recording that has it
    ("propertyIsBookkeepingV1", true), // not in the spec; null in the one recording that has it
    ("recurringIntervall", true), // not in the spec (double l, unlike `recurringInterval`); null in all 16 survey objects
    ("resultDisdar", true),       // not in the spec; null in all 16 survey objects
    ("sendType", true),           // not in the spec; null in all 16 survey objects
    ("taxmaroStockAccount", true), // not in the spec; null in all 16 survey objects
    ("vatNumber", true),          // not in the spec; null in all 16 survey objects
];

/// VoucherPos keys of the same kind.
const VOUCHER_POSITION_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 21 survey objects
    ("cateringTaxRate", true),       // not in the spec; null in all 21 survey objects
    ("createNextPart", true),        // not in the spec; null in all 21 survey objects
    ("specialAccountingField1", true), // not in the spec; null in all 21 survey objects
    ("specialAccountingField2", true), // not in the spec; null in all 21 survey objects
    ("specialAccountingField3", true), // not in the spec; null in all 21 survey objects
    ("specialAccountingField4", true), // not in the spec; null in all 21 survey objects
    ("specialAccountingField5", true), // not in the spec; null in all 21 survey objects
    ("sum", true),                   // not in the spec; null in all 21 survey objects
];

/// Document keys of the same kind (the spec has no schema for a document).
const DOCUMENT_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in both recordings
    ("description", true),           // not in the spec; null in both recordings
    ("documentNumber", true),        // not in the spec; null in both recordings
    ("mimeType", true),              // not in the spec; null in both recordings
];

/// VoucherLog keys of the same kind.
const VOUCHER_LOG_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in both recordings
];

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "voucher",
            unread::<Voucher>("voucher"),
            &[
                "objects.*.createUser.objectName",
                "objects.*.document.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "voucher_list",
            unread::<Voucher>("voucher_list"),
            &[
                "objects.*.createUser.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "sample_voucher_list",
            unread::<Voucher>("sample_voucher_list"),
            &[
                "objects.*.createUser.objectName",
                "objects.*.document.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
            ],
        ),
        (
            "voucher_positions",
            unread::<VoucherPosition>("voucher_positions"),
            &[
                "objects.*.accountDatev.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.voucher.objectName",
            ],
        ),
        (
            "sample_voucher_pos_list",
            unread::<VoucherPosition>("sample_voucher_pos_list"),
            &[
                "objects.*.accountDatev.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.voucher.objectName",
            ],
        ),
        (
            "write_voucher_reset_to_open",
            unread::<Voucher>("write_voucher_reset_to_open"),
            &[
                "objects.createUser.objectName",
                "objects.document.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
                "objects.taxRule.objectName",
            ],
        ),
        (
            "answer_voucher_resetToDraft",
            unread::<Voucher>("answer_voucher_resetToDraft"),
            &[
                "objects.createUser.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
                "objects.taxRule.objectName",
            ],
        ),
        (
            "write_voucher_book_amount",
            unread::<VoucherLog>("write_voucher_book_amount"),
            &[
                "objects.objectName",
                "objects.sevClient.objectName",
                "objects.voucher.objectName",
            ],
        ),
        (
            "write_voucher_save_create",
            unread::<SavedVoucher>("write_voucher_save_create"),
            &[
                "objects.voucher.createUser.objectName",
                "objects.voucher.objectName",
                "objects.voucher.sevClient.objectName",
                "objects.voucher.taxRule.objectName",
                "objects.voucherPos.*.accountDatev.objectName",
                "objects.voucherPos.*.objectName",
                "objects.voucherPos.*.sevClient.objectName",
                "objects.voucherPos.*.voucher.objectName",
            ],
        ),
        (
            "write_voucher_save_update_release_attach",
            unread::<SavedVoucher>("write_voucher_save_update_release_attach"),
            &[
                "objects.document.createUser.objectName",
                "objects.document.objectName",
                "objects.document.sevClient.objectName",
                "objects.document.updateUser.objectName",
                "objects.voucher.createUser.objectName",
                "objects.voucher.document.objectName",
                "objects.voucher.objectName",
                "objects.voucher.sevClient.objectName",
                "objects.voucher.taxRule.objectName",
                "objects.voucherPos.*.accountDatev.objectName",
                "objects.voucherPos.*.objectName",
                "objects.voucherPos.*.sevClient.objectName",
                "objects.voucherPos.*.voucher.objectName",
            ],
        ),
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "voucher" | "voucher_list" | "sample_voucher_list" => {
            under("objects.*.", VOUCHER_FORM_UNMEASURED)
        }
        "voucher_positions" | "sample_voucher_pos_list" => {
            under("objects.*.", VOUCHER_POSITION_FORM_UNMEASURED)
        }
        "write_voucher_reset_to_open" | "answer_voucher_resetToDraft" => {
            under("objects.", VOUCHER_FORM_UNMEASURED)
        }
        "write_voucher_book_amount" => under("objects.", VOUCHER_LOG_FORM_UNMEASURED),
        "write_voucher_save_create" | "write_voucher_save_update_release_attach" => {
            let mut keys = under("objects.voucher.", VOUCHER_FORM_UNMEASURED);
            keys.extend(under(
                "objects.voucherPos.*.",
                VOUCHER_POSITION_FORM_UNMEASURED,
            ));
            if fixture == "write_voucher_save_update_release_attach" {
                keys.extend(under("objects.document.", DOCUMENT_FORM_UNMEASURED));
            }
            keys
        }
        _ => return None,
    })
}
