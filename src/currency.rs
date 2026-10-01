//! The currency of an object that carries amounts: an ISO 4217 code.
//!
//! [`crate::Amount`] stays a plain cent value; the currency travels beside it
//! as its own field. No conversion, no exchange rates.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A three-letter ISO 4217 code such as `EUR`, exactly as sevDesk writes it.
///
/// Uppercase only: `"eur"` is rejected, not rewritten. Any well-formed code
/// is accepted, also one this crate has never seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrencyCode([u8; 3]);

impl CurrencyCode {
    pub const EUR: Self = Self(*b"EUR");

    /// `code` if it is exactly three ASCII uppercase letters.
    pub fn new(code: &str) -> Result<Self, InvalidCurrencyCode> {
        match code.as_bytes() {
            &[a, b, c] if [a, b, c].iter().all(u8::is_ascii_uppercase) => Ok(Self([a, b, c])),
            _ => Err(InvalidCurrencyCode(code.to_owned())),
        }
    }

    pub fn as_str(&self) -> &str {
        // Three ASCII letters by construction.
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

/// A string that is not three ASCII uppercase letters.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed currency code `{0}`: expected three uppercase letters (ISO 4217)")]
pub struct InvalidCurrencyCode(pub String);

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CurrencyCode {
    type Err = InvalidCurrencyCode;
    fn from_str(code: &str) -> Result<Self, Self::Err> {
        Self::new(code)
    }
}

impl Serialize for CurrencyCode {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CurrencyCode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let code = String::deserialize(d)?;
        Self::new(&code).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed_codes_are_kept_as_they_are() {
        for code in ["EUR", "CHF", "USD", "XAU", "ZZZ"] {
            assert_eq!(
                CurrencyCode::new(code).map(|c| c.to_string()),
                Ok(code.into())
            );
        }
        assert_eq!(CurrencyCode::new("EUR"), Ok(CurrencyCode::EUR));
    }

    #[test]
    fn lowercase_is_rejected_not_normalized() {
        for code in ["eur", "Eur", "euR"] {
            assert!(CurrencyCode::new(code).is_err(), "{code}");
        }
    }

    #[test]
    fn malformed_codes_are_rejected() {
        for code in ["", "EU", "EURO", "EU1", " EUR", "EUR ", "€", "ÄÖÜ"] {
            assert!(CurrencyCode::new(code).is_err(), "{code:?}");
        }
    }

    #[test]
    fn serde_round_trips_the_bare_string() {
        assert_eq!(
            serde_json::to_string(&CurrencyCode::EUR).unwrap(),
            "\"EUR\""
        );
        let back: CurrencyCode = serde_json::from_str("\"CHF\"").unwrap();
        assert_eq!(back.as_str(), "CHF");
        assert!(serde_json::from_str::<CurrencyCode>("\"chf\"").is_err());
        assert!(serde_json::from_str::<CurrencyCode>("null").is_err());
    }
}
