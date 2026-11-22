//! Check 2: spec properties a read type does not read, and spec properties it
//! reads but cannot decode from `null`.

use super::{exact, schema_properties};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value as Json};
use serde_yaml_ng::Value;
use sevdesk::*;
use std::collections::BTreeSet;

/// `schema.property`, with the reason the type leaves the property unread.
pub(super) const SPEC_FIELDS_NOT_READ: &[(&str, &str)] = &[
    (
        "Model_CheckAccountResponse.autoMapTransactions",
        "spec typo; the server field is autoMapTransaction (singular), read instead",
    ),
    (
        "Model_CheckAccountResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_CheckAccountTransactionResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_CommunicationWayResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_ContactAddressResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_ContactResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_InvoicePosResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_InvoiceResponse.datevConnectOnline",
        "an object without fields in the spec; in no recording",
    ),
    (
        "Model_InvoiceResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_VoucherPosResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_VoucherResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_creditNotePosResponse.objectName",
        "constant type tag, carries no data",
    ),
    (
        "Model_creditNoteResponse.objectName",
        "constant type tag, carries no data",
    ),
];

/// Properties the type requires, so the object decodes at all.
const PLACEHOLDERS: &[(&str, &str)] = &[("id", "1"), ("currency", "EUR")];

fn object(pairs: impl IntoIterator<Item = (String, Json)>) -> Json {
    Json::Object(Map::from_iter(pairs))
}

/// Decodes `value` into `T`, returning the keys `T` ignores.
fn decode<T: DeserializeOwned>(value: &Json) -> Result<Vec<String>, String> {
    let mut ignored = Vec::new();
    serde_ignored::deserialize::<_, _, T>(value, |path| ignored.push(path.to_string()))
        .map_err(|e| e.to_string())?;
    Ok(ignored)
}

/// Spec properties not read by `T`, and the properties `T` reads but cannot
/// decode from `null` (each with the error).
fn check<T: DeserializeOwned>(properties: &[String]) -> Result<(Vec<String>, Vec<String>), String> {
    let placeholders: Vec<(String, Json)> = PLACEHOLDERS
        .iter()
        .filter(|(k, _)| properties.iter().any(|p| p == k))
        .map(|(k, v)| ((*k).to_owned(), Json::String((*v).to_owned())))
        .collect();
    decode::<T>(&object(placeholders.clone()))
        .map_err(|e| format!("does not decode from the placeholders alone ({e})"))?;

    // A property the type reads but cannot take as `null` is a finding; it is
    // left out of the all-null object so the rest can still be measured.
    let mut null_failures = Vec::new();
    for property in properties {
        if placeholders.iter().any(|(k, _)| k == property) {
            continue;
        }
        let one = object(
            placeholders
                .iter()
                .cloned()
                .chain([(property.clone(), Json::Null)]),
        );
        if let Err(e) = decode::<T>(&one) {
            null_failures.push((property.clone(), format!("{property}: {e}")));
        }
    }
    let all_null = object(
        placeholders
            .iter()
            .cloned()
            .chain(
                properties
                    .iter()
                    .filter(|p| !null_failures.iter().any(|(f, _)| f == *p))
                    .filter(|p| !placeholders.iter().any(|(k, _)| k == *p))
                    .map(|p| (p.clone(), Json::Null)),
            )
            .collect::<std::collections::BTreeMap<_, _>>(),
    );
    let unread =
        decode::<T>(&all_null).map_err(|e| format!("does not decode with all null ({e})"))?;
    let null_failures = null_failures.into_iter().map(|(_, m)| m).collect();
    Ok((unread, null_failures))
}

type Checker = fn(&[String]) -> Result<(Vec<String>, Vec<String>), String>;

/// (spec schema, our read type).
fn mapping() -> Vec<(&'static str, Checker)> {
    vec![
        ("Model_CheckAccountResponse", check::<CheckAccount>),
        (
            "Model_CheckAccountTransactionResponse",
            check::<Transaction>,
        ),
        ("Model_ContactResponse", check::<Contact>),
        ("Model_ContactAddressResponse", check::<ContactAddress>),
        ("Model_CommunicationWayResponse", check::<CommunicationWay>),
        ("Model_creditNoteResponse", check::<CreditNote>),
        ("Model_creditNotePosResponse", check::<CreditNotePosition>),
        ("Model_InvoiceResponse", check::<Invoice>),
        ("Model_InvoicePosResponse", check::<InvoicePosition>),
        ("Model_VoucherResponse", check::<Voucher>),
        ("Model_VoucherPosResponse", check::<VoucherPosition>),
    ]
}

pub(super) fn problems(spec: &Value) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut out = Vec::new();
    for (schema, checker) in mapping() {
        match checker(&schema_properties(spec, schema)) {
            Ok((unread, null_failures)) => {
                found.extend(unread.iter().map(|p| format!("{schema}.{p}")));
                out.extend(
                    null_failures
                        .iter()
                        .map(|f| format!("{schema}: read property fails on null: {f}")),
                );
            }
            Err(e) => out.push(format!("{schema}: {e}")),
        }
    }
    out.extend(exact("SPEC_FIELDS_NOT_READ", &found, SPEC_FIELDS_NOT_READ));
    out
}
