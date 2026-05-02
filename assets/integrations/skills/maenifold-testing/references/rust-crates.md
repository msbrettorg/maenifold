# Rust Crate Verification

Use this reference for Rust build/test validation, compile regressions, release binaries, and crate-level runtime checks.

## Compile-regression checks

Filtered tests can hide compile failures in sibling modules. For code that changes public signatures or shared test fixtures, prefer:

```bash
cargo test -p <crate> --no-run
```

For task-specific verification, run the focused test after compile coverage is known.

## Delivery builds

Backend delivery requires release builds:

```bash
cargo build --release -p <crate>
```

Use the exact crate that owns the changed code. For maenifold-core tests, use `cargo test -p maenifold-core`; do not accidentally run tests against `maenifold-cli` when the unit tests live in core.

## Runtime freshness

If runtime behavior appears stale:

1. Build the relevant release binary from current source.
2. Re-test against that binary.
3. Compare size/hash/path with any packaged or daemon-loaded artifact.

Do not declare a task stale from old packaged behavior alone.
