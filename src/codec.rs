//! Scalars as sevDesk writes them, and as it wants them back.
//!
//! sevDesk is inconsistent inside one response: ids and status codes arrive as
//! strings (`"id": "6306017"`, `"status": "50"`), amounts as strings (`"57.49"`,
//! `"30"`) or as JSON numbers (`"paidAmount": 87.49`). Everything here reads
//! either form and never goes through a float: an amount is taken from the
//! SOURCE TEXT of its JSON value (`RawValue`) and parsed as integer cents.

use crate::Amount;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::value::RawValue;
use std::fmt;

/// `text` (a JSON number or a JSON string holding one) as cents.
///
/// `"30"`, `"57.49"`, `-100.32`, `"-0.350"` read; an exponent, a decimal comma
/// or a third significant decimal do not.
pub fn parse_amount(text: &str) -> Result<Amount, String> {
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text);
    if text.is_empty() || text.contains([',', 'e', 'E', '+']) {
        return Err(format!("unreadable amount `{text}`"));
    }
    // sevDesk sometimes pads (`"-0.350"`); trailing zeros carry no value.
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    };
    let text = if text.is_empty() || text == "-" {
        "0"
    } else {
        text
    };
    parse_cents(text)
        .map(Amount::from_minor)
        .ok_or_else(|| format!("unreadable amount `{text}`"))
}

/// An optional `-`, digits, optionally `.` and at most two more digits.
/// Integer arithmetic only.
fn parse_cents(text: &str) -> Option<i64> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let is_digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if !is_digits(whole) || !is_digits(fraction) || (whole.is_empty() && fraction.is_empty()) {
        return None;
    }
    if fraction.len() > 2 {
        return None;
    }
    let cents: i64 = format!("{whole}{fraction:0<2}").parse().ok()?;
    Some(if negative { -cents } else { cents })
}

/// An amount as the exact decimal string sevDesk takes: `-5749` is `"-57.49"`.
pub fn format_decimal(amount: Amount) -> String {
    let minor = amount.minor();
    let sign = if minor < 0 { "-" } else { "" };
    let abs = minor.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

pub(crate) fn amount_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Amount>, D::Error> {
    Option::<Box<RawValue>>::deserialize(d)?
        .map(|raw| parse_amount(raw.get()).map_err(de::Error::custom))
        .transpose()
}

pub(crate) fn serialize_amount<S: Serializer>(amount: &Amount, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&format_decimal(*amount))
}

/// An integer that arrives as a number or as a string holding one.
pub(crate) fn deserialize_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    struct V;
    impl Visitor<'_> for V {
        type Value = i64;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an integer or a string holding one")
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i64, E> {
            Ok(v)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i64, E> {
            i64::try_from(v).map_err(E::custom)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<i64, E> {
            v.trim().parse().map_err(E::custom)
        }
    }
    d.deserialize_any(V)
}

/// `Option` form of [`deserialize_i64`]; `null` is `None`.
pub(crate) fn deserialize_i64_opt<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<i64>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<i64>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("null, an integer or a string holding one")
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
            deserialize_i64(d).map(Some)
        }
    }
    d.deserialize_option(V)
}

/// A boolean that arrives as `true`, `"1"`, `1` (or the falsy forms).
pub(crate) fn serialize_flag<S: Serializer>(v: &bool, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(if *v { "1" } else { "0" })
}

/// A flag that arrives as `true`, `"1"`, `1` or `null`.
pub(crate) fn deserialize_flag_opt<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<bool>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<bool>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("null, a boolean, 0/1 or \"0\"/\"1\"")
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
            d.deserialize_any(self)
        }
        fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v != 0))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v != 0))
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            match v {
                "1" | "true" => Ok(Some(true)),
                "0" | "false" | "" => Ok(Some(false)),
                other => Err(E::custom(format!("unreadable flag `{other}`"))),
            }
        }
    }
    d.deserialize_option(V)
}

/// A percentage such as `"19"` or `"7.5"` in basis points (`1900`, `750`).
pub(crate) fn percent_bp_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    amount_opt(d).map(|v| v.map(Amount::minor))
}
