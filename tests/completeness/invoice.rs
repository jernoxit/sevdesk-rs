//! Completeness lists of the invoice resource group.

use super::under;
use super::{Covered, unread};
use sevdesk::*;

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

/// Paths where a recorded answer embeds a FULL object that the read type reads
/// as an `ObjectRef` (another call of the crate returns that object). All its
/// keys but the id are unread by design and collapse to one entry per path.
pub(super) fn embedded_objects(fixture: &str) -> &'static [&'static str] {
    match fixture {
        "write_invoice_cancel" => &["objects.contact", "objects.taxRule"],
        "write_invoice_save" | "quantity_probe_save" => {
            &["objects.invoicePos.*.invoice", "objects.invoicePos.*.unity"]
        }
        _ => &[],
    }
}

pub(super) fn covered() -> Vec<Covered> {
    vec![
        (
            "sample_invoice_list",
            unread::<Invoice>("sample_invoice_list"),
            &[
                "objects.*.addressCountry.objectName",
                "objects.*.contact.objectName",
                "objects.*.contactPerson.objectName",
                "objects.*.createUser.objectName",
                "objects.*.origin.objectName",
                "objects.*.paymentMethod.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.taxRule.objectName",
                "objects.*.objectName",
            ],
        ),
        (
            "sample_invoice_pos_list",
            unread::<InvoicePosition>("sample_invoice_pos_list"),
            &[
                "objects.*.invoice.objectName",
                "objects.*.objectName",
                "objects.*.sevClient.objectName",
                "objects.*.unity.objectName",
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
    ]
}

/// Full paths of this group's form-unmeasured keys in `fixture`, if its read
/// type has such a list.
pub(super) fn form_unmeasured(fixture: &str) -> Option<Vec<(String, bool)>> {
    Some(match fixture {
        "invoice_draft" | "invoice_enshrined" | "invoice_list" | "sample_invoice_list" => {
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
        "quantity_probe_get_invoice_pos" | "sample_invoice_pos_list" => {
            under("objects.*.", INVOICE_POSITION_FORM_UNMEASURED)
        }
        _ => return None,
    })
}
