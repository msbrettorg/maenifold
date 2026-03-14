# Agent Orchestration Guide

This document describes the multi-agent pipeline used for coordinating parallel work across SWE, red-team, and blue-team agents. The system was designed for test evaluation remediation but generalizes to any multi-stage review workflow.

## Architecture

```
┌─────────┐     ┌──────────┐     ┌───────────┐     ┌────────┐
│ pending │────▶│ SWE does │────▶│ Red-team  │────▶│Blue-   │────▶ done
│         │     │   work   │     │  audits   │     │team    │
└─────────┘     └──────────┘     └───────────┘     │verifies│
     ▲                                │            └───┬────┘
     │           REJECT               │                │
     └────────────────────────────────┘         REJECT │
     └─────────────────────────────────────────────────┘
```

Files move through four states: `pending → red → blue → done`. A rejection at any review stage returns the file to `pending` with an incremented attempt counter. After 3 failed attempts, a file is marked `stuck` for manual triage.

## Pipeline Database

State is tracked in a SQLite database rather than the filesystem. This eliminates race conditions between parallel agents and provides a single source of truth.

### Schema

```sql
CREATE TABLE eval_queue (
    file TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','red','blue','done','stuck')),
    attempts INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Key Queries

```sql
-- Dashboard: see pipeline state at a glance
SELECT status, count(*) FROM eval_queue GROUP BY status;

-- Get next file for SWE processing
SELECT file FROM eval_queue WHERE status = 'pending' ORDER BY file LIMIT 1;

-- Advance after SWE completes
UPDATE eval_queue SET status = 'red', updated_at = datetime('now') WHERE file = ?;

-- Advance after red-team passes
UPDATE eval_queue SET status = 'blue', updated_at = datetime('now') WHERE file = ?;

-- Advance after blue-team passes
UPDATE eval_queue SET status = 'done', updated_at = datetime('now') WHERE file = ?;

-- Reject: send back to pending
UPDATE eval_queue SET status = 'pending', attempts = attempts + 1,
    updated_at = datetime('now') WHERE file = ?;

-- Mark stuck after too many failures
UPDATE eval_queue SET status = 'stuck'
    WHERE attempts >= 3 AND status = 'pending';
```

## Agent Roles

### PM / Orchestrator

The PM never reads file contents or writes code. It:

1. Queries the DB for files at each status
2. Dispatches the matching agent type with the file path
3. Reads the agent's PASS/REJECT verdict
4. Updates the DB status accordingly
5. Immediately backfills any freed agent slot

### SWE Agent

Picks up `pending` files. Reads the eval spec, implements changes, runs targeted tests, commits. On success the PM advances the file to `red`.

### Red-team Agent

Picks up files at `red` status. Audits the SWE's changes against the eval spec. Read-only — never modifies code. Issues a PASS or REJECT verdict. Rejects go back to `pending`.

### Blue-team Agent

Picks up files at `blue` status. Runs the targeted tests to verify everything compiles and passes. Issues a PASS or FAIL verdict. Failures go back to `pending`.

## Streaming Dispatch

The critical operational rule: **never batch**. Do not wait for all N agents to finish before advancing any of them.

The correct pattern:

1. Poll all active agents
2. For each completed agent, immediately:
   - Read its result
   - Update the DB
   - Spawn the next-stage agent (or a new SWE if the file reached `done`)
3. Keep polling — the pipeline should always show a mix of SWE + red + blue running simultaneously on different files

This maximizes throughput. At steady state with 8 agent slots, you'll typically see 3-4 SWEs, 2-3 reds, and 1-2 blues running concurrently.

### Slot Management

Target: **8 concurrent agents**. When an agent completes:

1. Process its result (advance or reject in DB)
2. If the file advanced to a new stage, spawn that stage's agent
3. If a slot is still free, pull the next `pending` file for SWE
4. Never exceed the slot limit; never leave slots idle

## Agent Prompts

Each agent type has a prompt file that contains its full instructions:

| Agent | Prompt File | What It Contains |
|-------|-------------|-----------------|
| PM | `tests/evaluations/prompt.md` | DB queries, dispatch rules, slot management |
| SWE | `tests/evaluations/swe-prompt.md` | How to read eval specs, implement verdicts, run tests, commit |
| Red-team | `tests/evaluations/red-prompt.md` | How to audit SWE work, PASS/REJECT criteria |
| Blue-team | `tests/evaluations/blue-prompt.md` | How to verify tests pass, PASS/FAIL criteria |

### Prompt Design Principles

- **Agents are stateless.** Every prompt must be self-contained. An agent knows nothing about prior runs.
- **Agents pick from the DB.** The PM tells them which file to process; agents don't query the DB themselves.
- **Scoped testing only.** Agents must never run the full test suite. The prompt includes the exact command pattern for running only the relevant tests.
- **Append-only audit trail.** Each agent appends its section to the eval spec file, creating a running log: SWE remediation → red-team audit → blue-team verification.

## Test Binary Convention

This project uses a consolidated test binary (`integration_tests`) declared in `Cargo.toml`:

```toml
[[test]]
name = "integration_tests"
harness = true
```

Individual test files are `mod` includes via `tests/main.rs`. The correct invocation is:

```bash
cargo test --test integration_tests -p maenifold-core -- {module_name}
```

**Not** `cargo test --test {module_name}` (that looks for a standalone binary that doesn't exist).

This must be documented in every agent prompt. Agents that run `cargo test` without `--test integration_tests` will either fail or run the entire suite.

## Creating a New Pipeline

To set up this process for a new batch of work:

```bash
# 1. Create the database
sqlite3 pipeline.db "CREATE TABLE eval_queue (
    file TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','red','blue','done','stuck')),
    attempts INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);"

# 2. Load work items
for f in todo/*.md; do
    sqlite3 pipeline.db "INSERT INTO eval_queue (file) VALUES ('$(basename $f)');"
done

# 3. Verify
sqlite3 pipeline.db "SELECT count(*) FROM eval_queue WHERE status = 'pending';"
```

Then start the PM orchestrator, pointing it at the prompt files and database.

## Operational Notes

- **Agent timing**: Most agents complete in 60-200s. Complex evals can take 300-500s. If an agent exceeds 600s, it's likely stuck.
- **Zero-reject rate**: With well-written eval specs and good SWE prompts, expect >95% first-pass success.
- **Concurrency safety**: The PM is the single writer to the DB. Agents never touch it. This eliminates all concurrency issues without locks.
- **Gitignore the DB**: `pipeline.db` is operational state, not source code. Add it to `.gitignore`.
- **Model selection matters**: Use the strongest available model for red-team (catches subtle drift). SWE and blue-team can use faster models. See the model assignment table in the custom instructions.
