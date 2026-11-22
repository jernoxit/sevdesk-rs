//! An exact decimal for unit prices and quantities: integer mantissa plus
//! scale, no arithmetic, no rounding.
//!
//! Sums stay [`crate::Amount`] (cents). A unit price carries up to eight
//! fraction digits and a quantity is fractional (`"0.5"`), so neither fits
//! cents. The value is read from the SOURCE TEXT of the JSON value, never
//! through a float; whatever does not fit is an error, never rounded.

use std::fmt;
use std::str::FromStr;

/// An exact decimal number: `units * 10^-scale`, normalized so that `"10"`,
/// `"10.0"` and `10.00` are the same value (trailing fraction zeros are
/// stripped). The mantissa is an `i128`, i.e. 38 significant digits; the
/// measured values have at most 8 fraction digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Decimal {
    units: i128,
    scale: u32,
}

/// Text that is not a plain decimal or does not fit [`Decimal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDecimalError(String);

impl fmt::Display for ParseDecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseDecimalError {}

impl Decimal {
    /// `units * 10^-scale`, normalized: `Decimal::new(1000, 2)` is `10`.
    pub fn new(units: i128, scale: u32) -> Self {
        let (mut units, mut scale) = (units, scale);
        while scale > 0 && units % 10 == 0 {
            units /= 10;
            scale -= 1;
        }
        if units == 0 {
            scale = 0;
        }
        Self { units, scale }
    }

    /// The mantissa: `14.691264` is `14691264`.
    pub const fn units(self) -> i128 {
        self.units
    }

    /// The number of fraction digits: `14.691264` has `6`.
    pub const fn scale(self) -> u32 {
        self.scale
    }
}

impl FromStr for Decimal {
    type Err = ParseDecimalError;

    /// An optional `-`, digits, optionally `.` and at least one more digit.
    /// No `+`, no exponent, no comma, no whitespace.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || ParseDecimalError(format!("unreadable decimal `{text}`"));
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (whole, fraction) = match digits.split_once('.') {
            Some((whole, fraction)) => (whole, Some(fraction)),
            None => (digits, None),
        };
        let is_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if !is_digits(whole) || fraction.is_some_and(|f| !is_digits(f)) {
            return Err(bad());
        }
        let fraction = fraction.unwrap_or("");
        let scale = u32::try_from(fraction.len()).map_err(|_| bad())?;
        let units: i128 = format!("{whole}{fraction}").parse().map_err(|_| {
            ParseDecimalError(format!(
                "decimal `{text}` does not fit 38 significant digits"
            ))
        })?;
        Ok(Self::new(if negative { -units } else { units }, scale))
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.units < 0 { "-" } else { "" };
        let abs = self.units.unsigned_abs();
        if self.scale == 0 {
            return write!(f, "{sign}{abs}");
        }
        let divisor = 10_u128.pow(self.scale);
        write!(
            f,
            "{sign}{}.{:0width$}",
            abs / divisor,
            abs % divisor,
            width = self.scale as usize
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    #[test]
    fn reads_the_measured_forms_exactly() {
        assert_eq!(parse("14.691264"), Decimal::new(14691264, 6));
        assert_eq!(parse("-0.5"), Decimal::new(-5, 1));
        assert_eq!(parse("0.5"), Decimal::new(5, 1));
        assert_eq!(parse("-1"), Decimal::new(-1, 0));
        assert_eq!(parse("2"), Decimal::new(2, 0));
        assert_eq!(parse("0.12345678"), Decimal::new(12345678, 8));
        assert_eq!(parse("100.32").units(), 10032);
        assert_eq!(parse("100.32").scale(), 2);
    }

    #[test]
    fn trailing_zeros_normalize() {
        assert_eq!(parse("10"), parse("10.0"));
        assert_eq!(parse("10"), parse("10.00"));
        assert_eq!(parse("10").units(), 10);
        assert_eq!(parse("10.50"), parse("10.5"));
        assert_eq!(parse("0.0"), parse("0"));
        assert_eq!(parse("-0"), parse("0"));
        assert_eq!(parse("-0.00").scale(), 0);
        assert_eq!(Decimal::new(1000, 2), parse("10"));
    }

    #[test]
    fn displays_exactly() {
        for text in ["14.691264", "-0.5", "0.05", "-0.000001", "10", "-1", "0"] {
            assert_eq!(parse(text).to_string(), text);
        }
        assert_eq!(parse("10.00").to_string(), "10");
        assert_eq!(parse("-0.00").to_string(), "0");
    }

    #[test]
    fn keeps_38_digits_and_rejects_more() {
        let max = "99999999999999999999999999999999999999"; // 38 nines
        assert_eq!(parse(max).to_string(), max);
        assert!(
            "999999999999999999999999999999999999999"
                .parse::<Decimal>()
                .is_err()
        );
        assert!(
            "0.999999999999999999999999999999999999999"
                .parse::<Decimal>()
                .is_err()
        );
    }

    #[test]
    fn rejects_everything_that_is_not_a_plain_decimal() {
        for text in [
            "", "-", ".", "1.", ".5", "+1", "1e3", "1E3", "1,5", " 1", "1 ", "--1", "1.-5", "0x10",
            "NaN", "1.2.3", "true",
        ] {
            assert!(text.parse::<Decimal>().is_err(), "`{text}` must not read");
        }
    }
}
