---
name: maenifold-testing
description: Maenifold testing, debugging, and operational reference for the vanilla MCP-server baseline. Use this skill whenever the user asks to test, validate, reproduce, debug, or verify maenifold backend behavior; mentions the daemon, pipeline/driver behavior, MCP Gateway calls, build/package verification, or asks why a maenifold test/validation step failed. Also use when looking up LSP server status (rust-analyzer), LSP workarounds, command timeout budgets, no-polling-loop rules, or historical incident context (72-hour retrospective). Enforces source-backed debugging, runtime-binary verification, and an end-of-cycle retrospective that keeps this skill itself up to date.
---

# Maenifold Testing

This skill turns maenifold testing from improvisation into a documented loop. Use it to prove backend behavior in the real product, find the exact failure surface, and keep the testing playbook current as the repo changes.

## First move

Before running a test or debugging a failed test:

1. Read the repo-root `AGENTS.md` sections that apply to the surface under test.
2. If the work is RALPH-driven, read `ralph/AGENTS.md`, `ralph/state/thoughts.md`, and the relevant parts of `ralph/state/learnings.md`.
3. Classify the surface and read the focused reference file below. For mixed bugs, read every relevant reference.
4. State the success criterion in concrete terms: the exact command that must pass, the log/error that must disappear, the MCP response payload that must round-trip, or the data path that must be proven.

Do not use stale memory as a substitute for source. If behavior depends on a library or external protocol, fetch the canonical documentation/source first. Use Context7 for library/framework docs, GitHub for canonical repositories, and the repo source for maenifold behavior.

## Reference map

| Surface | Read |
|---|---|
| Pipeline/workflow behavior, driver processes, stale runtime binaries | `references/pipeline-workflow-drivers.md` |
| MCP/Gateway testing, curl helpers, gate calls | `references/mcp-gateway.md` |
| Rust crate build/test verification | `references/rust-crates.md` |
| LSP server status, config, per-server workarounds | `references/lsp-reference.md` |
| Command timeouts, per-tool budgets, no-polling clause | `references/command-timeouts.md` |
| 72-hour incident retrospective (historical) | `references/retrospective-72h.md` |
| RALPH autonomous testing cycles | `references/ralph-cycle.md` |
| End-of-cycle skill maintenance | `references/retrospective.md` |

## Error handling stance

An error is evidence, not permission to guess. When a command or validation step fails:

1. Capture the exact command, exit code, stdout/stderr, daemon log path, port, and PID where relevant.
2. Identify whether the failure is likely harness/process, environment/setup, product regression, stale runtime, or expected external dependency behavior.
3. Research the canonical source before proposing a fix: the relevant reference file, repo source, `ralph/state/learnings.md`, Context7, GitHub, or `GatewayIndex` depending on the surface.
4. Reduce the answer to one simple next action. If that action changes code or the testing harness, ask for a yes/no implementation decision unless the user already authorized it.

Avoid thrashing: do not try random flag combinations, alternate test tools, or speculative code edits. Resolve the class of failure from docs/source, then make the smallest correction that prevents repeating it.

## Keep this skill current

At the end of every testing/debug/UAT cycle, run `references/retrospective.md`.

If the cycle produced a reusable testing lesson, update the focused reference file before declaring the cycle complete. Keep `SKILL.md` small; put domain details in `references/`. Update `SKILL.md` only when the trigger description, routing table, or mandatory loop changes.

If there is no reusable lesson, say that explicitly in the cycle notes. Do not let the skill drift behind what actually works.

## Source map

- Repo testing constitution: `AGENTS.md`
- RALPH-specific rules: `ralph/AGENTS.md`
- Prior testing lessons: `ralph/state/learnings.md`
- Daemon redeploy: `rebuild.sh`
- Skill eval prompts: `evals/evals.json`
