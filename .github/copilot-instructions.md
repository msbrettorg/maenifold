# Copilot Instructions — maenifold-rs

## MANDATORY INSTRUCTIONS

You are required to use the 'gate' skill when working in this environment.
You are required to use 'gate append' to document your work and decisions.

If you are the product-manager agent you may also use gate pass and gate reject to manage work item lifecycle.
If you are not the product-manager use of those two tools is prohibited.

Do not use the built-in SQL `todos` table for task tracking. Gate is the authoritative system — `todo/` is the backlog, `done/` is completed work, `gate.db` is a rebuildable cache. The SQL session database may be used for ad-hoc queries (dependency graphs, batch analysis) but never as a source of truth for work item state.

## Glossary

| Term | What it is | Crate / Location |
|------|-----------|-------------------|
| **gate** | CLI task/sprint tracker. Manages todo items with pass/fail verdicts, agent dispatch, and file-authoritative state. Binary: `gate`. | `crates/maenifold-gate` |
| **pipeline** | DAG orchestrator MCP tool. Composes workflow nodes, sequential thinking, and one-shot commands into directed acyclic graphs with edges, concurrency, and lifecycle management. | `crates/maenifold-core` (workflow/pipeline/) |

These are completely different tools. Never conflate them. When the user says "pipeline" they mean the DAG orchestrator. When they say "gate" they mean the task tracker CLI.

## SDK and Dependency Policy

Never hand-roll functionality that an existing crate or SDK already provides. Specifically:

- **rmcp** — Use for all MCP server/transport concerns. Do not write custom MCP framing, tool registration, or transport layers.
- **acpx** — Use for ACP client/session management. Do not reimplement session lifecycle, message routing, or transport.
- If a well-maintained crate exists for the job, use it. Only write custom code for domain logic that no crate covers.

When in doubt, check `Cargo.toml` workspace dependencies first.

## Direct Commands

When told to delete, remove, revert, or perform any destructive action — execute it immediately. Do not:

- Audit dependencies first
- Investigate impact first
- Suggest alternatives
- Ask for confirmation (the user's message IS the confirmation)

The user makes architectural decisions. The agent executes them.

## Troubleshooting and Test Failures

When tests fail after a code change:

1. Investigate how to **reproduce** the issue
2. Investigate the **cause** of the issue
3. Investigate how to **resolve** the issue
4. Investigate the **impact** of the proposed resolution
5. Resolve the issue
6. Remediate any issues caused by the resolution, including test remediation

**DO NOT modify working code to fit broken tests.** If tests fail, the tests may be wrong. Investigate the test expectations against the intended behavior before changing production code.

## Sub-Agent Scope Discipline

Sub-agents must only modify files directly related to their assigned task. Do not refactor, test, or modify code in other crates unless explicitly asked. Commit only files related to the task. If you notice an issue in unrelated code, report it — do not fix it.

## No Parallel Agents on Same File

Never dispatch parallel sub-agents that touch the same file. Each agent must commit its changes before the next agent touches the same file. When planning parallel work, ensure non-overlapping file sets.

## Investigate Before Asking

Before asking the user a question, check if the answer is already in:

1. The current conversation
2. The codebase (grep, read files, check tests)
3. AGENTS.md, PRD.md, README.md, or other project docs

Only ask the user when the answer genuinely cannot be found through investigation.

## Build and Test

```bash
# Build release binary
cargo build --release -p maenifold-cli

# Build gate CLI
cargo build --release -p maenifold-gate

# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p maenifold-core

# Run a single test by name
cargo test -p maenifold-core test_name

# Makefile shortcut for release build
make release
```

Files must stay under 300 lines of code.

## Runtime Configuration

The maenifold root directory is `~/maenifold/` (not `~/.maenifold/`). Key runtime files:

- **`~/maenifold/agents.json`** — ACP agent definitions (Copilot, OpenCode, Goose, etc.). Loaded by `AgentsConfig::load()` from `Config::current().maenifold_root`.
- **`~/maenifold/memory/`** — Knowledge graph corpus
- **`~/maenifold/assets/`** — Workflow definitions, skills, integrations
- **`~/maenifold/memory.db`** — SQLite memory database

The root path can be overridden via `MAENIFOLD_ROOT` env var.
