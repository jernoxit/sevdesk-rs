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

<!-- Condensed from CLAUDE.md sections "External-Service APIs: typed boundaries, both directions" (incl. the 2026-11-22 rules on complete read types), "Intern und extern sind zwei Typen, verbunden durch `From`", "Fehler tragen Information, keine Handlungsanweisung"; CLAUDE.md remains the full version, both are kept in step. -->

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

## A response type maps every field its call returns

Every key of the recorded answers plus the fields the OpenAPI spec names, explored live where the shape is unclear (project owner, 2026-11-22); the consumer decides what it needs. For read fields this overrides "Simplicity First". An embedded full object stays an `ObjectRef` when another call of the crate returns that object, otherwise it becomes a read type of its own (widening a reference later is a type change). A field no measurement has shown filled gets the spec's type if the spec names one; with neither it stands on a named list "form unmeasured" until a measurement shows it (project owner, 2026-11-22). A completeness test without network keeps type and recordings in step: every key of every recorded answer is read or stands on a named list, compared by exact equality.

```text
bad:  Invoice reads 15 of the 90 keys of GET /Invoice/{id}; a consumer needs addressName -> release
good: every key is a field, or on the named list (embedded full objects)
```

## A closed set of wire values parses totally

Enums carry every value the spec names plus the measured ones; an unknown value (a spec typo, an unseen value) becomes a named catch-all arm, not an `Err`. A variant known only from the spec says so in its doc; where it is sent (filters, drafts), it is unmeasured.

```rust
// src/status.rs
Other(i64),   // from_code: code => Self::Other(code)
```

## A field value in a form no reader knows is a decode error naming the field

Never a silent `None`, which is indistinguishable from a real `null` (project owner, 2026-11-22). One broad reader per kind (switch, integer, timestamp, amount) in `src/codec.rs`, accepting every measured form.

```text
bad:  isPercentage "2" -> None
good: isPercentage "2" -> SevdeskError::Decode naming `isPercentage`
```

## Read types and their enums are `#[non_exhaustive]`

A field or variant added later is no breaking change (project owner, 2026-11-22). Not on query and draft types: there it would break `..Default::default()` for consumers.

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
