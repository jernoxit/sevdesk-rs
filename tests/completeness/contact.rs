//! Completeness lists of the contact resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

/// Contact keys that no recording or survey shows filled and that the OpenAPI
/// spec does not name (form unmeasured): not read until a measurement shows
/// them. The flag says whether any fixture carries the key at all.
const CONTACT_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("mandateReference", true), // not in the spec; null in all 130 survey objects
    ("taxType", true),          // not in the spec; null in all 130 survey objects
];

/// ContactAddress keys of the same kind.
const CONTACT_ADDRESS_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 35 survey objects
];

/// CommunicationWay keys of the same kind.
const COMMUNICATION_WAY_FORM_UNMEASURED: &[(&str, bool)] = &[
    ("additionalInformation", true), // not in the spec; null in all 35 survey objects
];

/// Paths where a recorded answer embeds a FULL object that the read type reads
/// as an `ObjectRef` (another call of the crate returns that object). All its
/// keys but the id are unread by design and collapse to one entry per path.
pub(super) fn embedded_objects(fixture: &str) -> &'static [&'static str] {
    match fixture {
        "contact_address" | "write_contact_address_create" => &["objects.contact"],
        _ => &[],
    }
}

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "communication_way",
            unread::<CommunicationWay>("communication_way"),
            &[
                "objects.contact.objectName",
                "objects.key.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "communication_way_list",
            unread::<CommunicationWay>("communication_way_list"),
            &[
                "objects.*.contact.objectName",
                "objects.*.key.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "contact",
            unread::<Contact>("contact"),
            &[
                "objects.*.category.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "contact_list_by_customer_number",
            unread::<Contact>("contact_list_by_customer_number"),
            &[
                "objects.*.category.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "contact_address",
            unread::<ContactAddress>("contact_address"),
            &[
                "objects.category.objectName",
                "objects.contact.<embedded object>",
                "objects.country.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "contact_address_list",
            unread::<ContactAddress>("contact_address_list"),
            &[
                "objects.*.category.objectName",
                "objects.*.contact.objectName",
                "objects.*.country.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "sample_communication_way_list",
            unread::<CommunicationWay>("sample_communication_way_list"),
            &[
                "objects.*.contact.objectName",
                "objects.*.key.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "sample_contact_address_list",
            unread::<ContactAddress>("sample_contact_address_list"),
            &[
                "objects.*.category.objectName",
                "objects.*.contact.objectName",
                "objects.*.country.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "sample_contact_list",
            unread::<Contact>("sample_contact_list"),
            &[
                "objects.*.category.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
            ],
        ),
        (
            "write_communication_way_create",
            unread::<CommunicationWay>("write_communication_way_create"),
            &[
                "objects.contact.objectName",
                "objects.key.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "write_contact_address_create",
            unread::<ContactAddress>("write_contact_address_create"),
            &[
                "objects.category.objectName",
                "objects.contact.<embedded object>",
                "objects.country.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
        (
            "write_contact_create",
            unread::<Contact>("write_contact_create"),
            &[
                "objects.category.objectName",
                "objects.objectName",
                "objects.sevClient.objectName",
            ],
        ),
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "contact" | "contact_list_by_customer_number" | "sample_contact_list" => {
            under("objects.*.", CONTACT_FORM_UNMEASURED)
        }
        "write_contact_create" => under("objects.", CONTACT_FORM_UNMEASURED),
        "contact_address_list" | "sample_contact_address_list" => {
            under("objects.*.", CONTACT_ADDRESS_FORM_UNMEASURED)
        }
        "contact_address" | "write_contact_address_create" => {
            under("objects.", CONTACT_ADDRESS_FORM_UNMEASURED)
        }
        "communication_way_list" | "sample_communication_way_list" => {
            under("objects.*.", COMMUNICATION_WAY_FORM_UNMEASURED)
        }
        "communication_way" | "write_communication_way_create" => {
            under("objects.", COMMUNICATION_WAY_FORM_UNMEASURED)
        }
        _ => return None,
    })
}
