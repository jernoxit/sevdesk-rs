---
paths:
  - "src/**/*.rs"
  - "tests/**/*.rs"
---

<!-- Condensed from CLAUDE.md section "Dateizuschnitt: von Anfang an trennen, nicht später zerlegen"; CLAUDE.md remains the full version, both are kept in step. -->

# Module layout

## A module expected to exceed 500 lines starts as a directory module

If the design already shows a module will pass 500 lines, create it as a directory with separate files at once; "split later" rarely happens and every diff on the big file stays wide.

```text
bad:  src/vouchers.rs, planned at 800 lines
good: src/wire/mod.rs (doc, mod, re-exports) · src/wire/voucher.rs · src/wire/invoice.rs
```

## A section banner is a file, not a comment

`// ==== Vouchers ====` and `// ==== Invoices ====` mean the cut lines are known; cut along them.

```rust
// bad
// ==== Vouchers ====
// ==== Invoices ====
// good: one file per resource, as in src/endpoints/voucher.rs and src/endpoints/invoice.rs
```

## A `mod.rs` is doc, `mod` declarations and re-exports, nothing else

Logic landing in a facade means the split is overdue.

```rust
// src/endpoints/mod.rs
//! The endpoint methods of [`crate::SevdeskClient`], one file per resource.

mod check_account;
mod contact;
```

## A trait and its implementations live in separate files

This is a layout rule: it says where a trait goes, not whether there is one. Each implementation gets its own file.

```text
bad:  trait Provider + both impls in one file
good: provider.rs (trait) · provider/live.rs · provider/fake.rs
```

## A trait needs one of four reasons, and the reason is in its doc comment

"Looks cleaner" is no reason; a test double as a side effect is neither a reason for nor against. The four reasons: (1) exchange: a nameable second implementation that is promised or foreseeably needed, even if it does not exist yet; (2) dependency inversion: one module must not name another, so the trait states only what that one edge needs; (3) type erasure: different types of the same shape behind a runtime lookup; (4) the outside boundary: sevDesk's own shape stays internal, the public API speaks in this crate's types.

```rust
// src/wire/guard.rs
/// A trait for type erasure (reason 3): one drift check over many wire types
/// of the same form, each naming its own load-bearing fields.
pub(crate) trait Guard {
```
