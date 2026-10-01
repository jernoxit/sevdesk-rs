/// Which fields of a wire type carry the meaning; see the module doc.
///
/// A trait for type erasure (reason 3): one drift check over many wire types
/// of the same form, each naming its own load-bearing fields.
pub(crate) trait Guard {
    /// Names the object in a drift error.
    const NAME: &'static str;
    /// `true` when at least one load-bearing field (besides the id) is present.
    fn load_bearing_present(&self) -> bool;
}
