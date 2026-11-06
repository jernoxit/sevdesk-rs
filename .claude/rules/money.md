---
paths:
  - "src/amount.rs"
  - "src/codec.rs"
  - "src/payload.rs"
  - "src/wire/**/*.rs"
  - "src/document_payload/**/*.rs"
  - "src/endpoints/**/*.rs"
  - "tests/fixtures.rs"
  - "tests/requests.rs"
---

<!-- Condensed from CLAUDE.md section "Geld rechnet ganzzahlig in Cent und rundet kaufmännisch"; CLAUDE.md remains the full version, both are kept in step. -->

# Money

## Every amount is an integer number of cents

The type is `Amount` (`src/amount.rs`), a signed `i64` of cents.

## Every rounding goes half away from zero, also for negative amounts

This is the commercial convention; round-half-to-even is wrong for money.

## Shares and percentages compute in integers, never in `f64`

`1 − 0.30` lies just below `0.7` as a double, so an exact half cent silently rounds down.

```rust
// bad
let net = (price_cents as f64 * (1.0 - 0.30)).round() as i64;
// good: percentages are integer basis points (see percent_bp_opt in src/codec.rs)
let net = round_half_away(price_cents * (10_000 - 3_000), 10_000);   // integer numerator and divisor, no truncating `/`
```

## There is exactly one rounding function for money

If the crate needs a rounding, it is written once; a second formula creates a second truth.

## At the sevDesk boundary amounts go through exact decimal text, never `f64`

The API takes numbers but returns some amounts as strings (`"-100.32"`). Write with exact decimal formatting from cents, read with exact parsing to cents.

```rust
// src/codec.rs
pub fn format_decimal(amount: Amount) -> String   // -5749 -> "-57.49"
pub fn parse_amount(text: &str) -> Result<Amount, String>
```
