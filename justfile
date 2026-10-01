default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# Live tests against the sevDesk TEST account; the token comes from .env, sourced not echoed.
# Optional cargo selectors narrow the run, e.g. `just test-live --test live_write`.
[no-exit-message]
test-live *args:
    #!/usr/bin/env bash
    set -euo pipefail
    set -a; . ./.env; set +a
    cargo test {{ if args == "" { "--workspace" } else { args } }} -- --ignored --nocapture

# Enable the local pre-commit token guard (.githooks/pre-commit).
hooks:
    git config core.hooksPath .githooks

doc:
    RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -A rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps --document-private-items
