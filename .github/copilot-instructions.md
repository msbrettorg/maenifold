# Agent Team — maenifold-rs

**This file is the authoritative source for agent operating rules.** Process changes, dispatch policies, and lessons learned go HERE — not in memory://, not in session state. This file is durable and read by every agent on every session.

## Test Execution Policy

**SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
**Red-team agents:** Static analysis only — read code and reason. Never execute tests.
**Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
**PM:** One `cargo test -p <crate>` at sprint close to confirm green. Never mid-sprint.

Agents running full test suites in parallel is a waste and creates race conditions. Violating this policy is grounds for the PM to kill the agent immediately.

## Red-team and Blue-team Scoping

**Red-team:** Audit ONLY the files modified by the SWE for this task. Pre-existing issues in untouched files are out of scope — note them as informational, do NOT FAIL. Verdict is on the DELTA, not the codebase.

**Blue-team:** Run ONLY tests that exercise code changed by this task. If test compilation fails due to unrelated code from other tasks, report the blockage and recommend waiver — do NOT treat unrelated compilation failures as task failures.

**Sub-PMs:** Maximum 2 remediation cycles per task. If blue-team reports unrelated compilation failure, escalate to PM immediately. Do NOT dispatch another remediation SWE for someone else's breakage.

## Research Before Building

When given a new feature area, research existing ecosystems and libraries FIRST. Never propose building infrastructure that an established project already provides. If the user names a specific ecosystem (e.g., "use Codex plugins"), that means become a consumer of it — do not reimplement its capabilities. Hand-rolling something that a mature library or platform already solves is a P0 escalation.

## Visual Verification Required

For UI changes, `vue-tsc --noEmit` passing is necessary but NOT sufficient. Vue `<style scoped>` does NOT apply to child component internals — every new Vue component MUST have its own `<style>` block with its own styles. UI work is not done until visually verified (Playwright screenshot, browser automation, or human confirmation). A feature that typechecks but renders at 0px is a failed delivery.

## No Over-Parallelization

One sub-PM per crate. Never run parallel SWE agents on files in the same Rust crate — they conflict at `cargo build` time even on different files because `target/` is workspace-global. Stagger by crate boundary. Blue-team verification is only meaningful after all SWEs on a crate have completed. Cap sub-PM task lists to 4-5 items to avoid spawning 30+ agents.

## Full-Stack Spec Verification

Before implementing, verify the spec covers the FULL STACK for the feature. Frontend-only changes that require backend support are no-ops. If a spec mentions UI controls for a backend capability, confirm the backend fields, wire format, and execution path exist first. A frontend selector with no backend plumbing is not a feature — it is a lie.

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

## Gate CLI Fallback

Gate MCP sessions go stale frequently ('Session not found'). Always be prepared to fall back to CLI:

```bash
gate sync --root . --rebuild    # rebuild read model from task files
gate status --root .            # show all pending tasks
gate pass <task-id> --agent <name> --notes "..." --root .
gate fail <task-id> --agent <name> --notes "..." --root .
```

Prefer Gate MCP when healthy, but do not retry more than once — switch to CLI immediately.
