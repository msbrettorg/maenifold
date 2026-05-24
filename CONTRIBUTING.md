# Contributing to maenifold-rs

## 1) Quick orientation

Start with the [README](./README.md) for product context, platform scope, and local-first architecture before making changes.

## 2) Architecture map

This repository is a Rust workspace exposing maenifold as a vanilla MCP server:

- `crates/maenifold-core` — core domain logic (tools, memory, graph, workflows, ACP/MCP orchestration)
- `crates/maenifold-cli` — `maenifold` CLI + MCP server binary entrypoint
- `crates/maenifold-gate-core` — shared gate lifecycle logic exposed through the `maenifold.gate` Gateway domain
- `crates/maenifold-jockey` — coordination/runtime crate in the workspace
- `crates/maenifold-pipeline-driver` — headless pipeline execution binary

### MCP-only boundary rule

Keep domain behavior behind MCP/tool surfaces in `maenifold-core`. Clients and binaries should orchestrate and transport, not duplicate domain policy.

## 3) Build from source

### Prerequisites

- Rust stable toolchain
- Git

Optional for full feature coverage:

- ONNX model under `assets/models/` for semantic/vector features
- vec0 native library under `assets/native/<platform>/`

### Rust workspace build

From repo root:

```bash
cargo build
```

Targeted builds:

```bash
cargo build -p maenifold-core
cargo build -p maenifold-cli
```

### Release redeploy

```bash
./rebuild.sh
```

Builds every crate in release, verifies the `/usr/local/bin/maenifold` symlink, and `launchctl kickstart`s the daemon on macOS.

## 4) Testing

Run the narrowest relevant test first, then expand:

```bash
cargo test -p maenifold-core
cargo test -p maenifold-cli
cargo test --test workspace_cli_smoke
MAENIFOLD_FULL_SUITE=1 cargo test
```

Testing philosophy is strict: **no mocks, stubs, or fakes** for core behavior. Prefer real SQLite, real filesystem interactions, and real integration paths. See [docs/TESTING_PHILOSOPHY.md](./docs/TESTING_PHILOSOPHY.md).

## 5) Code conventions

- **Mechanism, not policy**: provide composable primitives and explicit behavior; avoid hidden “smart” automation.
- Keep modules focused: one file, one primary responsibility.
- Keep files small and maintainable: target a hard ceiling of **~300 LOC per file** (split sooner when cohesion drops).
- Avoid unnecessary abstraction layers and speculative optimizations.
- Keep error details explicit and actionable.

Reference philosophy:
- [docs/MA_MANIFESTO.md](./docs/MA_MANIFESTO.md)
- [docs/WHAT_WE_DONT_DO.md](./docs/WHAT_WE_DONT_DO.md)
- [docs/ma/WHAT_WE_DONT_DO.md](./docs/ma/WHAT_WE_DONT_DO.md)

## 6) Gate workflow (SDD)

Gate is file-authoritative. Task markdown in `todo/` and `done/` is source-of-truth; `.gate/gate.db` is rebuildable cache.

Default SDD stage flow:

`todo/ (pending) → swe → red-team → blue-team → done/`

Typical loop:

1. `GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo"}`
2. `GatewayExecute domain="maenifold.gate" command="gate_Status" parameters={root: "/path/to/repo"}`
3. implement/review work for current stage
4. record with `gate_Append`, then advance with `gate_Pass` or return with `gate_Fail`

Spec reference: [docs/gate-uml-spec.md](./docs/gate-uml-spec.md)

## 7) Where to start

If you're new:

1. Build the workspace successfully.
2. Run a focused test in the area you plan to change.
3. Read the crate/module you will touch end-to-end before editing.
4. Make minimal, scoped changes that preserve existing boundaries.
5. Re-run focused tests, then broader checks if needed.
6. Update docs when behavior or workflows change.

Good first contribution targets:

- Small tool-surface fixes in `maenifold-core`
- CLI/help-text and error-quality improvements
- Focused tests that increase real-behavior coverage
- Documentation clarifications across `docs/`

## 8) Documentation index

Repository docs in `docs/*.md`:

- [docs/README.md](./docs/README.md)
- [docs/BOOTSTRAP.md](./docs/BOOTSTRAP.md)
- [docs/DEVELOPMENT.md](./docs/DEVELOPMENT.md)
- [docs/MA_MANIFESTO.md](./docs/MA_MANIFESTO.md)
- [docs/SCRIPTING.md](./docs/SCRIPTING.md)
- [docs/SECURITY_MODEL.md](./docs/SECURITY_MODEL.md)
- [docs/TESTING_PHILOSOPHY.md](./docs/TESTING_PHILOSOPHY.md)
- [docs/WHAT_WE_DONT_DO.md](./docs/WHAT_WE_DONT_DO.md)
- [docs/acp-client-architecture.md](./docs/acp-client-architecture.md)
- [docs/acp-golden-tests.md](./docs/acp-golden-tests.md)
- [docs/acp-gpt54-thoughtlevel-timing.md](./docs/acp-gpt54-thoughtlevel-timing.md)
- [docs/gate-uml-spec.md](./docs/gate-uml-spec.md)
