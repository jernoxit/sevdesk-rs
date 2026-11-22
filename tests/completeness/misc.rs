//! Completeness lists of the misc resource group.

use super::{Covered, unread};
use sevdesk::*;

pub(super) fn covered() -> Vec<Covered> {
    vec![
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
            "write_upload_temp_file",
            unread::<UploadedFile>("write_upload_temp_file"),
            &[
                "objects.content",
                "objects.contentHash",
                "objects.isEInvoice",
            ],
        ),
    ]
}
