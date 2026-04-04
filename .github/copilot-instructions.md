# Agent Team — maenifold-rs


## Test Execution Policy

**SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
**Red-team agents:** Static analysis only — read code and reason. Never execute tests.
**Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
**PM:** One `cargo test -p <crate>` at sprint close to confirm green. Never mid-sprint.

Agents running full test suites in parallel is a waste and creates race conditions. Violating this policy is grounds for the PM to kill the agent immediately.

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

## Web UI / ACP Rules

- WebSocket URI must include `?agent=<name>`. Never connect without it.
- No default agent fallback. Missing `?agent=` must be rejected with an error.
- PM smoke tests must connect to a named agent. Bare `/ws/acp` is a test failure.
