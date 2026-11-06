---
paths:
  - "src/wire/**/*.rs"
  - "src/document_payload/**/*.rs"
  - "src/endpoints/**/*.rs"
  - "src/client.rs"
  - "src/codec.rs"
  - "src/error.rs"
  - "src/status.rs"
  - "src/payload.rs"
  - "src/query.rs"
---

<!-- Condensed from CLAUDE.md sections "External-Service APIs: typed boundaries, both directions", "Intern und extern sind zwei Typen, verbunden durch `From`", "Fehler tragen Information, keine Handlungsanweisung"; CLAUDE.md remains the full version, both are kept in step. -->

# sevDesk API boundary

## Every request and response is a typed struct

Both directions go through `Serialize`/`Deserialize` structs: no `json!()` bodies, no `serde_json::Value` field-poking, no ad-hoc exception. If sevDesk needs a non-standard encoding, the typed param struct owns it in one place.

```rust
// bad
let body = serde_json::json!({ "contact": { "id": id } });
// good: a struct in src/wire/ or src/document_payload/, e.g. InvoiceWire in src/document_payload/invoice.rs
```

## Deserialization is tolerant

`Option` fields and no `deny_unknown_fields`: foreign APIs grow fields.

## A payload missing all load-bearing fields is an error, never a defaulted object

Each wire type implements `Guard`; a decoded object with none of its load-bearing fields present becomes `SevdeskError::DriftGuard`.

```rust
// bad
let invoice: InvoiceWire = serde_json::from_str(&body)?;   // all-None object passes
// good: the check in src/wire/guard.rs, error in src/error.rs
```

## A closed set of wire values parses totally

An unknown value becomes a named catch-all arm, not an `Err`.

```rust
// src/status.rs
Other(i64),   // from_code: code => Self::Other(code)
```

## Where a measurement contradicts the OpenAPI spec, the measurement wins

The spec has wrong enums and a misspelled filter. Anchor each shape with a live-captured payload under `tests/fixtures/`.

## The API version is pinned by an explicit `X-Version` header on every request

```rust
// src/client.rs
headers.insert(
    "X-Version",
    HeaderValue::from_str(version).map_err(|_| invalid("X-Version"))?,
);
```

## An error response does not mean "not written"

sevDesk answers 500 and 400 with the write done, and errors inside HTTP 200: check every body for `error`, and re-read before retrying after any failure. The only exception is HTTP 429, which the rate limiter rejects before the service: it is retried directly, writes included (`src/client.rs`, `src/pacing.rs`).

```text
bad:  500 on a write -> send the same write again
good: 500 on a write -> read the object back, then decide
      429 on a write -> retry directly (rejected before the service)
```

## Wire types and public types are two types, connected by `From`

The conversion destructures the source type completely (`let Source { a, b, c } = v;`, no `..`, target in field shorthand), so a new field is a compile error exactly where someone must decide if it is needed. If the orphan rule rejects `From` (E0117), a named function with the reason at it stands there.

```rust
// src/document_payload/invoice.rs
impl From<InvoiceDraft> for InvoiceWire {
    fn from(draft: InvoiceDraft) -> Self {
        let InvoiceDraft { contact, contact_person, /* ... */ is_e_invoice } = draft;
```

## An error says what happened, not what to do

The crate has its own error type (`SevdeskError`); whether to retry, report or give up is the consumer's decision from the variant. Because `?` converts silently through `From`, verify a change of error types by counting and reading the conversions, not by "it compiles".

```rust
// bad
#[error("rate limit hit, please retry later")]
// good — src/error.rs
#[error("sevDesk rate limit persisted after {attempts} attempts")]
RateLimited { attempts: u32 },
```
