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
cargo build -p maenifold-pipeline-driver
```

If your change touches `crates/maenifold-core/src/workflow/`, build both
`maenifold-cli` and `maenifold-pipeline-driver`: the CLI hosts the daemon
surface, and the driver hosts pipeline execution.

## Format / Lint

```bash
cargo fmt --all
make quality-lint
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
MAENIFOLD_FULL_SUITE=1 cargo test
```

### Vector / embedding suite

```bash
MAENIFOLD_VECTOR_SUITE=1 cargo test -p maenifold-core --test vector_integration_tests
```

### Single exact test

```bash
cargo test --test workspace_cli_smoke workspace_cli_smoke_is_rust_only -- --exact
```

## Quality reports

Install the extra analysis tools when you want coverage and code-health reports:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov cargo-machete rust-code-analysis-cli
npm install -g jscpd
```

Then run the focused targets from the repository root:

```bash
make quality-coverage       # LCOV summary under target/quality/coverage/
make quality-coverage-html  # HTML coverage report
make quality-metrics        # rust-code-analysis JSON metrics
make quality-dead-code      # clippy dead/unused findings as JSON lines
make quality-unused-deps    # cargo machete report
make quality-duplicates     # jscpd duplicate-code report
make quality-report         # aggregate all of the above + approximate file-level CRAP
```

Notes:
- `quality-lint` keeps correctness-oriented Clippy checks as a gate while letting the dedicated quality report own complexity/size signals such as `too_many_lines`, `too_many_arguments`, and `cognitive_complexity`.
- `quality-report` opts into the full Rust suite explicitly via `MAENIFOLD_FULL_SUITE=1` when generating coverage.
- The vector/embedding suite now lives in `vector_integration_tests` and stays opt-in unless you set `MAENIFOLD_VECTOR_SUITE=1` or target an exact vector test by name.
- `quality-dead-code` intentionally scopes Clippy to library/binary targets so production-code dead/unused findings stay usable even when test-only modules are noisy.
- The CRAP section is file-level and approximate: it combines LCOV line coverage with `rust-code-analysis` complexity metrics.
- Raw artifacts land under `target/quality/`.

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
maenifold tool <name> '<json>'
```

Request JSON output when needed:

```bash
maenifold tool <name> '<json>' --json
```
