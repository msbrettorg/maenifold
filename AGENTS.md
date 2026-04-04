# Agent Team — maenifold-rs

We're building and dogfooding `maenifold` and `gate` at the same time. 
Any issues encountered with usability, semantics, errors, documentation, workflow, naming conventions, etc. should be escalated to the product-manager and added to the backlog using `gate` as a new T- item.
Red-team audit items must ALWAYS be added to `gate` regardless of severity.
Always check for newly added or pre-existing items before adding new.
Default to the TDD process when using `gate`.

## Test Execution Policy

**SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
**Red-team agents:** Static analysis only — read code and reason. Never execute tests.
**Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
**PM:** One `cargo test -p <crate>` at sprint close to confirm green. Never mid-sprint.

Agents running full test suites in parallel is a waste and creates race conditions. Violating this policy is grounds for the PM to kill the agent immediately.

We use Azure AI foundry as the backend for Codex and OpenCode in this environment. 
Copilot CLI uses GitHub Enterprise for the backend.
Codex, OpenCode and Copilot CLI have all been verified as working and have parity in features for our purposes.
Auth issues with ACP Agents = configuration problems with the code.

## Build

```bash
cargo build --release -p maenifold-cli     # CLI/app changes
cargo build --release -p maenifold-gate    # gate changes
make app                                    # frontend + CLI (touch lib.rs forces re-embed)
```

Binary: `target/release/maenifold` → symlinked at `/usr/local/bin/maenifold`.
Binary: `target/release/gate` for gate runtime changes.

**Words are not delivery. A rebuilt, runnable binary is delivery.**

## ACP Architecture

The ACP client architecture is specified in [docs/acp-client-architecture.md](docs/acp-client-architecture.md). That document is law — do not invent architecture outside it.

Key invariant: **`prompt` is the only primitive.** Think and workflow are orchestration loops that chain prompt calls. Neither should have a direct path to the ACP agent.

## ACP Model Policy

All ACP agent interactions MUST specify `model` explicitly. Never rely on agent defaults — different agents default to different models with wildly different latency and cost profiles.

The only valid ACP invocation path is `AcpExec` (via MCP `--tool` or the MCP protocol). There are no CLI aliases. `model` is a required parameter with no default.

## ACP Golden Tests (AcpExec E2E)

See [docs/acp-golden-tests.md](docs/acp-golden-tests.md) for the full test spec covering prompt, think, and workflow modes with memory artifact verification.

## Web UI / ACP Rules

- WebSocket URI must include `?agent=<name>`. Never connect without it.
- No default agent fallback. Missing `?agent=` must be rejected with an error.
- PM smoke tests must connect to a named agent. Bare `/ws/acp` is a test failure.
