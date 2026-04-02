# Agent Team — maenifold-rs

At session start you are required to read /Users/brett/src/ma-collective/maenifold-rs/docs/MA_MANIFESTO.md and /Users/brett/src/ma-collective/maenifold-rs/docs/WHAT_WE_DONT_DO.md. This is not optional. This is the reason we're building this repo to start with - it's our constitution.

## Roles

| Role | Model | Purpose |
|------|-------|---------|
| **PM** | `claude-opus-4.6-1m` | Sprint orchestration, gate.db state, PRD/RTM ownership, branch/release management |
| **SWE** | `gpt-5.4` | Focused implementation — code edits, features, bug fixes, refactoring |
| **Red-team** | `claude-sonnet-4.6` | Adversarial code audit — logic flaws, spec violations, drift detection |
| **Blue-team** | `gpt-5.4` | Verification — named test execution, coverage gates, PASS/FAIL verdicts |
| **Researcher** | `claude-opus-4.6` | Deep synthesis — technical research, fact-checking, graph-grounded findings |

## Dispatch Rules

- 1 focused task per agent. Scope to a single file or behaviour.
- Every `task` call MUST include `model` explicitly. No defaults.
- Never use `claude-opus-4.6` or `claude-opus-4.6-1m` without explicit user permission (Researcher excepted).
- Always include `[[WikiLinks]]` in prompts for concept-as-protocol context injection.
- SWE and Red-team must not modify files outside their assigned scope.
- No parallel agents on the same file. Commit before the next agent touches it.

## File Mandate

- Every file has exactly one responsibility. No mixed responsibilities in a single file.
- File names and paths must describe exactly what the file is in plain English.
- Files must be self-documenting by structure and naming. Do not rely on vague containers or catch-all files.
- The moment a file is touched, this mandate applies in full to that file. Do not make partial compliance edits.
- Old untouched files may remain as they are. Changed files must go the whole hog: single responsibility, explicit naming, exact placement.

## Test Execution Policy

**SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
**Red-team agents:** Static analysis only — read code and reason. Never execute tests.
**Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
**PM:** One `cargo test -p <crate>` at sprint close to confirm green. Never mid-sprint.

Agents running full test suites in parallel is a waste and creates race conditions. Violating this policy is grounds for the PM to kill the agent immediately.

## Operating Rules

- **Answer questions immediately.** No tool calls, no side work. Answer, then ask if they want you to continue.
- **P0 escalation:** any code violating Ma principles (silent defaults, hidden recovery, fake confidence) — stop, report to PM, create backlog item. Non-negotiable.

## Banned Models

| Model | Reason |
|-------|--------|
| `gemini-*` | File locking, false test failures |
| `claude-haiku-*` | 75% false positive rate on code audits |
| `gpt-5.1-codex-mini` | Misses drift that Sonnet 4.6 catches |

## Build

```bash
cargo build --release -p maenifold-cli     # CLI/app changes
make app                                    # frontend + CLI (touch lib.rs forces re-embed)
```

Binary: `target/release/maenifold` → symlinked at `/usr/local/bin/maenifold`.

**Words are not delivery. A rebuilt, runnable binary is delivery.**

## ACP Model Policy

All ACP agent interactions MUST specify `model: gpt-5.4` unless explicitly overridden. Never rely on agent defaults — different agents default to different models with wildly different latency and cost profiles.

CLI commands (`think`, `prompt`, `workflow`) default to `gpt-5.4`. Pipeline nodes should specify model in their node definition.

## Web UI / ACP Rules

- WebSocket URI must include `?agent=<name>`. Never connect without it.
- No default agent fallback. Missing `?agent=` must be rejected with an error.
- PM smoke tests must connect to a named agent. Bare `/ws/acp` is a test failure.
