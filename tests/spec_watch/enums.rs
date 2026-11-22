//! Check 3: spec enum values that a read enum of ours maps to `Other(..)`.

use super::{exact, property_enum};
use serde::de::DeserializeOwned;
use serde_yaml_ng::Value;
use sevdesk::*;
use std::collections::BTreeSet;
use std::fmt::Debug;

/// `schema.property=value`, with the reason the value stays `Other`.
pub(super) const SPEC_ENUM_VALUES_AS_OTHER: &[(&str, &str)] = &[];

/// Decodes each value into `E`; those landing in `Other(..)` come back, a
/// value that fails to decode as an error text.
fn other_values<E: DeserializeOwned + Debug>(values: &[String]) -> Result<Vec<String>, String> {
    let mut other = Vec::new();
    for value in values {
        let decoded: E = serde_json::from_value(serde_json::Value::String(value.clone()))
            .map_err(|e| format!("{value:?} does not decode: {e}"))?;
        if format!("{decoded:?}").starts_with("Other(") {
            other.push(value.clone());
        }
    }
    Ok(other)
}

type Decoder = fn(&[String]) -> Result<Vec<String>, String>;

/// (spec schema, spec property, our read enum).
fn mapping() -> Vec<(&'static str, &'static str, Decoder)> {
    vec![
        (
            "Model_CheckAccountResponse",
            "type",
            other_values::<CheckAccountType>,
        ),
        (
            "Model_CheckAccountResponse",
            "importType",
            other_values::<CheckAccountImportType>,
        ),
        (
            "Model_CheckAccountResponse",
            "status",
            other_values::<CheckAccountStatus>,
        ),
        (
            "Model_CheckAccountTransactionResponse",
            "status",
            other_values::<TransactionStatus>,
        ),
        (
            "Model_CommunicationWayResponse",
            "type",
            other_values::<CommunicationWayType>,
        ),
        (
            "Model_creditNoteResponse",
            "status",
            other_values::<CreditNoteStatus>,
        ),
        (
            "Model_creditNoteResponse",
            "sendType",
            other_values::<DocumentSendType>,
        ),
        (
            "Model_InvoiceResponse",
            "status",
            other_values::<InvoiceStatus>,
        ),
        ("Model_InvoiceResponse", "taxType", other_values::<TaxType>),
        (
            "Model_InvoiceResponse",
            "invoiceType",
            other_values::<InvoiceType>,
        ),
        (
            "Model_InvoiceResponse",
            "sendType",
            other_values::<DocumentSendType>,
        ),
        (
            "Model_VoucherResponse",
            "status",
            other_values::<VoucherStatus>,
        ),
        (
            "Model_VoucherResponse",
            "creditDebit",
            other_values::<CreditDebit>,
        ),
        (
            "Model_VoucherResponse",
            "voucherType",
            other_values::<VoucherType>,
        ),
        (
            "Model_VoucherResponse",
            "recurringInterval",
            other_values::<RecurringInterval>,
        ),
    ]
}

pub(super) fn problems(spec: &Value) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut out = Vec::new();
    for (schema, property, decode) in mapping() {
        match decode(&property_enum(spec, schema, property)) {
            Ok(values) => {
                found.extend(values.iter().map(|v| format!("{schema}.{property}={v}")));
            }
            Err(e) => out.push(format!("{schema}.{property}: enum value {e}")),
        }
    }
    out.extend(exact(
        "SPEC_ENUM_VALUES_AS_OTHER",
        &found,
        SPEC_ENUM_VALUES_AS_OTHER,
    ));
    out
}
