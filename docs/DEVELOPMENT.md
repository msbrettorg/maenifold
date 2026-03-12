<!-- T-10.14: RTM FR-16 / RTM-093 -->
# Development Guide

Rust-specific build, test, and contribution notes for `maenifold-rs`.

## Prerequisites

- Rust stable toolchain
- Git
- Optional: ONNX model under `assets/models/` for semantic/vector features
- vec0 native library for your platform under `assets/native/<platform>/`

Notes:
- The workspace builds without an ONNX model, but semantic features degrade cleanly when no model is present.
- vec0 is only needed for vector-backed features that rely on SQLite vector tables.

## Build

From the repository root:

```bash
cargo build
cargo build -p maenifold-core
cargo build -p maenifold-cli
```

## Format / Lint

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## Test

Tests are heavy. Follow the repo policy in `AGENTS.md`: run the narrowest relevant test first, then expand only as needed.

### Unit / boundary

```bash
cargo test -p maenifold-core
```

### CLI

```bash
cargo test -p maenifold-cli
```

### Workspace integration

```bash
cargo test --test workspace_cli_smoke
```

### Full suite

```bash
cargo test
```

### Single exact test

```bash
cargo test --test workspace_cli_smoke workspace_cli_smoke_is_rust_only -- --exact
```

## Workspace structure

- `crates/maenifold-core/` — core library: tool handlers, registry, MCP server, storage, search, vector integration
- `crates/maenifold-cli/` — binary entrypoint wrapping `maenifold_core::CliApp`
- `tests/` — Rust-native workspace integration tests
- `assets/` — workflows, tool docs, models, and native dependencies

## Adding a new tool

1. Add the tool to `crates/maenifold-core/src/registry.rs`.
2. Implement the handler in the appropriate core module.
3. Add a boundary test under `crates/maenifold-core/tests/`.
4. Add a CLI test under `crates/maenifold-cli/tests/`.
5. Add a workspace integration test under `tests/` when the change crosses crate boundaries.
6. Run the narrowest relevant test commands first, then rerun broader suites as a checkpoint.

If the tool has usage/help text, keep the tool contract, JSON envelope behavior, and error wording aligned with existing registry patterns.

## MCP mode

Run the stdio MCP server:

```bash
maenifold --mcp
```

## CLI mode

Run a single tool directly:

```bash
maenifold --tool <name> --payload '<json>'
```

Request JSON output when needed:

```bash
maenifold --tool <name> --payload '<json>' --json
```
