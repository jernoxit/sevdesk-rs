---
paths:
  - "tests/**/*.rs"
  - "src/currency.rs"
  - "src/pacing.rs"
---

<!-- Condensed from CLAUDE.md sections "Live-Tests nur auf ausdrücklichen Aufruf", "Bestandsdeckel: eine Zahl, die nur sinken darf", "Die Mutationsprobe beweist, dass ein Test eine Zusage bewacht"; CLAUDE.md remains the full version, both are kept in step. -->

# Tests

## A test that reaches sevDesk is `#[ignore]` and runs only through `just test-live`

It runs against the test account, never with a production token; `just test` stays green without network and without a token.

```rust
// tests/live.rs
#[ignore = "needs SEVDESK_API_KEY"]
```

## A rule with existing stock is capped by a hygiene test compared by exact equality, never `<=`

The rule holds at once for everything new and everything touched; the rest is capped by a named number or list. Growth fails (the purpose) and a drop fails too, with the new number in the message, so it is lowered in the same commit; otherwise every clean-up silently makes room for new violations. A cap at zero is a ban.

```rust
// bad
assert!(found <= CAP);
// good
assert_eq!(found, CAP, "now {found}: lower CAP to {found} in this commit");
```

## A test guards a promise only if weakening the promise turns it red

Weaken the promise, see the test fail, restore the file. If the promise hangs on two places, weakening one stays green: a green probe is a finding, not an acquittal.

```text
1. weaken the promise in src/
2. cargo test <filter> -> expect red
3. git checkout -- <file>; then check mechanically that no weakening is left in the tree
```

## Run the filtered test first, the full suite only on green

Red is a complete result; a filtered green only means the filter was wrong. Rerun a red test filtered: if it turns green unchanged, it is flaky and gets reported.

```text
bad:  cargo test --workspace   (to find out whether the weakened promise is caught)
good: cargo test --test fixtures amounts_read_exactly
```
