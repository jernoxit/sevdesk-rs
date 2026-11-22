default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

# `--all-targets` skips every `[[test]]` declared `test = false` (the live probes),
# so a second pass lints them explicitly. Their names are read from Cargo.toml, so
# a new probe cannot be forgotten; the guard fails if the extraction finds none.
lint:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo clippy --workspace --all-targets -- -D warnings
    probes=$(awk '/^name = /{n=$3; gsub(/"/,"",n)} /^test = false/{print "--test " n}' Cargo.toml)
    test -n "$probes" || { echo "no test = false targets found in Cargo.toml" >&2; exit 1; }
    cargo clippy --workspace $probes -- -D warnings

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

# Downloads sevDesk's public OpenAPI spec (static file, no token) and lists what is
# new relative to the read types and endpoints. The spec itself is never committed.
spec-check:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/tmp/spec
    curl -fsS -D target/tmp/spec/headers.txt -o target/tmp/spec/openapi.yaml https://api.sevdesk.de/openapi.yaml
    shasum -a 256 target/tmp/spec/openapi.yaml
    grep -i '^last-modified' target/tmp/spec/headers.txt || echo "no Last-Modified header"
    cargo test --test spec_watch -- --ignored --nocapture

# Enable the local pre-commit token guard (.githooks/pre-commit).
hooks:
    git config core.hooksPath .githooks

doc:
    RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -A rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps --document-private-items
