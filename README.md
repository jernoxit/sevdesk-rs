# sevdesk-rs

> **Unofficial.** This is an independent, community-style client. It is not
> affiliated with, endorsed by or supported by sevDesk GmbH. "sevDesk" is used
> only to name the API this crate talks to.

A typed Rust client (crate `sevdesk`) for the sevDesk REST API v1. It covers
check accounts and their transactions, vouchers, invoices, credit notes,
contacts and a few lookups. Every request and response
is a typed struct; nothing is assembled from `json!()` or picked out of a
`serde_json::Value`.

The client was shaped by measurements against a real sevDesk test account.
Where a measurement contradicts sevDesk's OpenAPI spec, the measurement wins.

## What differs from the spec (measured)

- **Errors inside HTTP 200.** sevDesk answers some failures with 200 and an
  `error` object, so every body is checked for one, whatever the status. A
  500 or 400 does not mean the write did not happen: read back before you
  retry.
- **HTTP 429 is the exception.** The rate limiter rejects the request before
  the service sees it, so the client retries a 429 itself (writes included),
  with `Retry-After` or exponential backoff and adaptive spacing (AIMD). No
  429 was reached in measurement, so a fresh client sends without spacing.
- **`X-Version` is pinned** to `default` on every request.
- **Filters fail silently.** A misspelled or unknown filter is ignored and the
  call returns everything. The spec's `customerIntenalNote` is such a typo;
  the client sends `customerInternalNote`, which matches EXACTLY (and finds an
  invoice together with its cancellation invoice). `paymtPurpose` and
  `descriptionLike` match SUBSTRINGS. `isBooked=false` is ignored; filter on
  `status` instead.
- **`autoMapTransaction`** on a check account is singular; the spec's plural
  is ignored.
- **Deleted transactions** are still returned by id, with `deletedAt` set; the
  list omits them.
- **Booking amounts carry the sign of the transaction:** −58.31 pays a credit
  note of 58.31.
- **Only drafts change.** `saveVoucher` updates only drafts (400, code 90400)
  and only a draft voucher deletes (409, code 159); a transaction deletes only
  in status 100 (409, code 159). On a voucher update every position goes along
  with its id, because sevDesk totals only the positions of that call.
- **Documents get their number on `sendBy`.** `saveInvoice` makes a draft
  without a number; `sendBy` with `VPDF` enshrines it and assigns the number.
  A credit note needs a number from its sequence and a `bookingCategory`.
- **E-invoices render only with a number.** `getPdf` and `changeParameter` on
  an e-invoice DRAFT fail with 503 ("Invoice number is required"); enshrine
  first. An e-invoice credit note needs a `paymentMethod`, else rendering it
  answers 830. `customerNumber` and `contact[id]` filters are exact, and
  `changeParameter` `en_US` alone makes the PDF English (no render call).
- **The transaction log** (`CheckAccountTransactionLog`, undocumented) filters
  by transaction but not by document; it pages stably with `limit`/`offset`.
- **Days are cut in Berlin time** by `getBalanceAtDate`, and `dd.mm.yyyy`
  voucher dates come back as Berlin midnight.
- **`currency` is always there** on check accounts, vouchers, invoices and
  credit notes, although the spec allows `null` on vouchers and credit notes.
  Transactions, transaction log entries, voucher positions and bookings carry
  none.

The crate docs (`just doc`, then `target/doc/sevdesk/index.html`) carry these
details at the types and methods they concern.

## Amounts and currency

Amounts are `sevdesk::Amount`: signed integer cents, read exactly from the
source text of a JSON number or string and written as exact decimal strings.
No floats anywhere. The type has no arithmetic; convert at your boundary with
`Amount::from_minor` and `Amount::minor`.

An amount carries no currency; the object does. Check accounts, vouchers,
invoices and credit notes have `currency: sevdesk::CurrencyCode` beside their
amounts, and a response without one is an error, never a default. A
transaction's amount is in its check account's currency, a voucher position's
in its voucher's. The drafts (`InvoiceDraft`, `CreditNoteDraft`,
`VoucherDraft`) carry the currency as a field; there is no EUR default.
`CurrencyCode` is three uppercase letters (ISO 4217): `"eur"` is rejected,
an unknown but well-formed code is kept. No conversion, no exchange rates.

Only EUR is measured. Documents also carry `sum*ForeignCurrency` (not read);
whether a document in another currency reports `sumGross` in that currency or
in the account's is open, and a voucher in another currency needs an exchange
rate this crate does not send.

## Usage

```rust
use sevdesk::{
    ApiToken, CurrencyCode, InvoiceQuery, SevdeskClient, SevdeskClientConfig, SevdeskError,
};

async fn euro_invoices_for(
    token: String,
    reference: &str,
) -> Result<Vec<sevdesk::Invoice>, SevdeskError> {
    let client = SevdeskClient::new(SevdeskClientConfig::new(ApiToken::new(token)))?;
    let invoices = client
        .list_invoices(&InvoiceQuery {
            customer_internal_note: Some(reference.into()),
            ..Default::default()
        })
        .await?;
    // Amounts are measured for EUR documents only.
    Ok(invoices
        .into_iter()
        .filter(|invoice| invoice.currency == CurrencyCode::EUR)
        .collect())
}
```

The library reads no environment variables: the token, base URL, pacing and
`X-Version` come in through `SevdeskClientConfig`.

## Minimum supported Rust version

The MSRV is **Rust 1.88** (`rust-version` in `Cargo.toml`). Cargo refuses to
build the crate with an older toolchain.

The MSRV is raised only when a dependency or a language feature requires it,
and the raise is noted in the commit message. The CI job `msrv` checks it
(`cargo check --all-targets --locked` on exactly that version). Renovate
updates the development toolchain (`rust-toolchain.toml`), not the MSRV.

## Development

The toolchain is pinned in `rust-toolchain.toml`; [`just`](https://just.systems)
runs everything CI runs:

```sh
just fmt-check   # rustfmt
just lint        # clippy, warnings denied
just test        # unit tests and mock-server tests (wiremock, recorded fixtures)
just doc         # rustdoc, broken intra-doc links denied
```

### Token guard (optional)

`just hooks` enables `.githooks/pre-commit`, which refuses a commit whose
staged diff contains the `SEVDESK_API_KEY` from your `.env`, or that stages a
recorded response with an unredacted account object (`lastLoginIp`). The token
is never printed.

### Live tests

The live tests need **your own sevDesk TEST account** and its API token; they
talk to that account and are `#[ignore]`d, so
neither `cargo test` nor CI runs them. To run them, put the test account's
token into `.env` (see `.env.example`; the file is git-ignored) and use:

```sh
just test-live                      # all ignored tests: reads and writes
just test-live --test live          # read-only calls
just test-live --test live_write    # writes, one test at a time, swept before and after
```

The recipe sources `.env` into the test process; the test harness reads
`SEVDESK_API_KEY` from there, and a test without it skips with a message. The
tests rely on fixed ids of the test account and the write tests delete what
carries their `SS-LIVE-` marker: never give them a production token.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
