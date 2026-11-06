---
paths:
  - "src/**/*.rs"
  - "tests/**/*.rs"
---

<!-- Condensed from CLAUDE.md section "Die Bibliothek liest keine Umgebung — Werte werden gereicht"; CLAUDE.md remains the full version, both are kept in step. -->

# Library reads no environment

## There is no `env::var` in `src/`

Token, base URL and spacing all arrive as parameters from the consumer; an environment variable in a library is a hidden input that a test can only set process-wide.

```rust
// bad
let token = std::env::var("SEVDESK_API_KEY")?;
// good: src/client.rs
SevdeskClient::new(SevdeskClientConfig::new(api_token))
```

## Several values that belong together cross the boundary as one type

A config type goes over the border instead of a row of bare scalars.

```rust
// bad
SevdeskClient::new(token, base_url, spacing_ms, x_version)
// good: SevdeskClientConfig in src/client.rs
```

## Only the live-test harnesses under `tests/` read `SEVDESK_API_KEY`

`just test-live` sets it from `.env`; no other test reads the environment.

```rust
// tests/live_write/harness.rs
let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
```
