//! The amount type of this crate: integer cents, no arithmetic, no rounding.
//!
//! Its own type rather than a money crate's, so callers depend on no second
//! crate to use the client; an adapter converts through [`Amount::minor`] and
//! [`Amount::from_minor`].

/// An amount in cents (two decimal places), signed as sevDesk carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Amount(i64);

impl Amount {
    pub const fn from_minor(minor: i64) -> Self {
        Self(minor)
    }

    /// The amount in cents.
    pub const fn minor(self) -> i64 {
        self.0
    }
}
