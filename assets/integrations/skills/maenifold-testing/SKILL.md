---
name: maenifold-testing
description: Maenifold testing, UAT, and debugging workflow. Use this skill whenever the user asks to test, UAT, validate, reproduce, debug, or verify maenifold behavior; mentions the packaged Electron app, agent-browser, CDP, reconnect, ACP sessions, pipeline/driver behavior, red browser errors, desktop regressions, build/package verification, or asks why a maenifold test/validation step failed. Enforces the canonical scripts/uat-launch.sh path, source-backed debugging, real packaged-app verification, and an end-of-cycle retrospective that keeps this skill itself up to date.
---

# Maenifold Testing

This skill turns maenifold testing from improvisation into a documented loop. Use it to prove behavior in the real product, find the exact failure surface, and keep the testing playbook current as the repo changes.

## First move

Before running a test or debugging a failed test:

1. Read the repo-root `AGENTS.md` sections that apply to the surface under test.
2. If the work is RALPH-driven, read `ralph/AGENTS.md`, `ralph/state/thoughts.md`, and the relevant parts of `ralph/state/learnings.md`.
3. Classify the surface and read the focused reference file below. For mixed bugs, read every relevant reference.
4. State the success criterion in concrete terms: the UI text/control that must appear, the exact command that must pass, the log/error that must disappear, or the data path that must be proven.

Do not use stale memory as a substitute for source. If behavior depends on a library or external protocol, fetch the canonical documentation/source first. Use Context7 for library/framework docs, GitHub for canonical repositories, and the repo source for maenifold behavior.

## Reference map

| Surface | Read |
|---|---|
| Packaged Electron UI, CDP, agent-browser, visual UAT | `references/packaged-electron-uat.md` |
| ACP sessions, reconnect, replay, bridge behavior | `references/acp-sessions.md` |
| Desktop frontend build, Vue/Tailwind/design audits | `references/frontend-build.md` |
| Pipeline/workflow behavior, driver processes, stale runtime binaries | `references/pipeline-workflow-drivers.md` |
| MCP/Gateway testing, curl helpers, gate calls | `references/mcp-gateway.md` |
| Rust crate build/test verification | `references/rust-crates.md` |
| RALPH autonomous testing cycles | `references/ralph-cycle.md` |
| End-of-cycle skill maintenance | `references/retrospective.md` |

## Error handling stance

An error is evidence, not permission to guess. When a command, app action, or validation step fails:

1. Capture the exact command, exit code, stdout/stderr, visible UI state, console errors, app log path, port, and PID where relevant.
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
- UAT launcher: `scripts/uat-launch.sh`
- Skill eval prompts: `evals/evals.json`
