---
name: test-driven-development
description: The team's test-driven development process. Use this skill when the user asks to "start a sprint", "run TDD", "set up a pipeline", "dispatch agents", "coordinate parallel work", "run eval specs", "process specs through agents", mentions "pipeline.db", "eval_queue", "tasks table", "work_log", "SWE/red/blue pipeline", "sprint planning", "requirements traceability", or wants to orchestrate subagents through the implement → audit → verify workflow. Also use when the user says "TDD", "test-driven", or references PRD/RTM/TODO traceability.
---

# Test-Driven Development Process

Everything is a file. The database is a rebuildable cache.

A spec file is a unit of work. It lives in `todo/`. Agents read it, do the work, append their results. When the pipeline finishes with it, the `advance` subcommand moves it to `done/`. Markdown task files under `todo/` and `done/` are authoritative. `pipeline.db` is only a fast query cache rebuilt from disk whenever needed.

## Philosophy

The unix view: small files, each representing one task, flowing through a pipeline of transforms. The cache is `ps` — it gives you fast queryability into what's running and where. The files are the filesystem — they hold the actual content and accumulate history. `todo/` is the inbox. `done/` is the archive. Agents are processes.

- **Files are authoritative.** Task state lives in markdown files under `todo/` and `done/`.
- **`pipeline.db` is a rebuildable cache.** Use it for fast queries, but treat it as disposable; `gate sync --rebuild .` recreates it from disk at any time.
- **The `advance` subcommand enforces legal transitions** and appends the canonical `SWE Remediation`, `Red-team Audit`, and `Blue-team Verification` sections before updating the cache.
- **`gate status` reports effective state derived from files,** with optional cache cross-checks when you pass `--db`.
- **`gate sync` rebuilds or reconciles the cache from disk.**

## Requirements Hierarchy

Every line of code traces back to a requirement through two documents and the filesystem:

```
PRD.md (FR-*, NFR-*)  →  RTM.md (FR-* → Component → TC-*)  →  todo/*.md (T-* specs)
```

| What | Where | Format |
|------|-------|--------|
| **Requirements** | `PRD.md` (pipeline root) | `FR-N: System SHALL...` / `NFR-N: System SHALL...` |
| **Traceability** | `RTM.md` (pipeline root) | `FR-N → file/component → TC-N → test type` |
| **Backlog** | `todo/` (one file per task) | Each file is a T-* spec, filename is the task ID |
| **Completed work** | `done/` (moved from todo/) | Same files, now carrying full audit trail |

There is no TODO.md. `ls todo/` is the backlog. `ls done/` is completed work. `wc -l todo/* done/*` is your sprint progress. The filesystem is the source of truth for what's left and what's finished.

Only the PM adds/modifies requirements (PRD.md, RTM.md) or moves files to `done/`.

## The Pipeline

```
todo/T-1.1-auth-login-jwt.md ──▶ SWE ──▶ Red-team ──▶ Blue-team ──▶ done/T-1.1-auth-login-jwt.md
      ▲                      │                │
      │        REJECT        │         REJECT │
      └──────────────────────┘                │
      └───────────────────────────────────────┘
```

Three stages. Each agent appends its section to the file. Rejections return to pending — the next SWE attempt sees the rejection rationale in the file. On completion, the file moves from `todo/` to `done/` carrying the full audit trail: spec → SWE work → red-team audit → blue-team verification.

### The agents

| Agent | Reads | Does | Appends |
|-------|-------|------|---------|
| SWE | spec + any prior rejections | implements, tests, commits | `## SWE Remediation` |
| Red-team | spec + SWE section | audits SWE against spec | `## Red-team Audit` (PASS/REJECT) |
| Blue-team | spec + SWE + red sections | runs scoped tests | `## Blue-team Verification` (PASS/FAIL) |

### The cache database

Four tables. Files on disk are the artifacts and source of truth. The DB is the index + state-machine cache.

```sql
-- Requirements from PRD.md
CREATE TABLE requirements (
    id TEXT PRIMARY KEY,          -- 'FR-1', 'NFR-1'
    type TEXT NOT NULL CHECK(type IN ('FR','NFR')),
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Traceability from RTM.md
CREATE TABLE traceability (
    test_case TEXT PRIMARY KEY,   -- 'TC-1'
    requirement TEXT NOT NULL REFERENCES requirements(id),
    component TEXT NOT NULL,      -- 'src/auth/'
    test_type TEXT NOT NULL,      -- 'unit', 'integration', 'benchmark'
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Task queue — each row points to a file in todo/ or done/
CREATE TABLE tasks (
    file TEXT PRIMARY KEY,        -- 'T-1.1-auth-login-jwt.md'
    title TEXT,
    requirement TEXT NOT NULL REFERENCES requirements(id),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','red','blue','done','stuck')),
    location TEXT NOT NULL DEFAULT 'todo'
        CHECK(location IN ('todo','done')),
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Work log — append-only record of every agent action
CREATE TABLE work_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file TEXT NOT NULL REFERENCES tasks(file),
    stage TEXT NOT NULL CHECK(stage IN ('swe','red','blue')),
    agent_model TEXT,             -- 'gpt-5.4', 'claude-sonnet-4.6'
    verdict TEXT,                 -- 'PASS', 'REJECT', 'FAIL', NULL for SWE
    duration_seconds INTEGER,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

`requirements` and `traceability` mirror the PRD.md and RTM.md on disk. They provide the relational backbone for cache-backed reporting, but the markdown files remain canonical.

`tasks` points to files. The `location` column tracks whether the file is in `todo/` or `done/`. The `requirement` FK connects every task back to the PRD.

`work_log` is append-only. Every agent action gets a row in the cache. This is the audit trail in queryable form — the file on disk has the same info in markdown, but the cache lets you ask "what's our average red-team reject rate?" without parsing markdown.

The PM is the only writer. Agents never touch the cache directly; the PM uses the `advance`, `status`, and `sync` subcommands as the canonical interface.

## Directory Layout

```
{pipeline_root}/
├── PRD.md                # Requirements (FR-*, NFR-*)
├── RTM.md                # Traceability matrix (FR-* → TC-*)
├── todo/                 # Backlog — pending spec files
├── done/                 # Archive — completed specs with audit trails
├── pipeline.db           # Rebuildable cache/index (gitignored)
├── swe-prompt.md         # SWE agent instructions
├── red-prompt.md         # Red-team agent instructions
└── blue-prompt.md        # Blue-team agent instructions
```

`PRD.md` and `RTM.md` are the contracts — what to build and how to trace it. `todo/` and `done/` are the work. All four are committed. `pipeline.db` is a rebuildable cache, gitignored.

Sprint planning = write spec files into `todo/`, each referencing FR-* from PRD.md. Sprint progress = `ls todo/ | wc -l` remaining, `ls done/ | wc -l` complete. Sprint close = `todo/` is empty.

## Setup

A working reference scaffold is in `references/scaffold/` — a ready-to-copy pipeline root with a seeded cache, sample PRD/RTM, one todo item, and one done item. Copy it, adapt it, go.

```bash
# Copy scaffold to your pipeline root
cp -r references/scaffold/ /path/to/pipeline/

# Or start from scratch:
mkdir -p todo done
# Create PRD.md, RTM.md (see scaffold for examples)

# Rebuild pipeline.db from the authoritative files on disk
gate sync --rebuild .

# Write agent prompts (copy from references/, adapt test commands)
echo "pipeline.db" >> .gitignore
```

### Gate binary location

`gate` is built from `crates/maenifold-gate`. Put the binary on your `PATH` to use the short `gate ...` form in prompts/scripts. During development, use `cargo run -p maenifold-gate -- <subcommand>`. The release binary is `target/release/gate`.

## The PM Dispatch Loop

Five things, forever:

1. **Query** `gate status` for files at each status
2. **Dispatch** the matching agent with the file path
3. **Read** the agent's verdict
4. **Advance or reject** with the `advance` subcommand; on blue PASS it moves the file to `done/`
5. **Backfill** any freed slot immediately

### Queries

#### Dispatch operations

```bash
# Dashboard
gate status --root .

# Machine-readable queue snapshot
gate status --root . --json

# Next file for SWE
# Filter the JSON array for the first object whose status == "pending"
gate status --root . --json

# Advance: SWE done → red
gate advance T-1.1-auth-login-jwt.md --root . --stage swe \
  --notes "Implemented feature and ran scoped tests" \
  --model "gpt-5.4"

# Advance: red passed → blue
gate advance T-1.1-auth-login-jwt.md --root . --stage red --verdict PASS \
  --notes "Spec-compliant after audit" \
  --model "claude-sonnet-4.6"

# Advance: blue passed → done (moves file to done/)
gate advance T-1.1-auth-login-jwt.md --root . --stage blue --verdict PASS \
  --notes "Scoped verification passed" \
  --model "gpt-5.4"

# Reject: red sends file back to pending
gate advance T-1.1-auth-login-jwt.md --root . --stage red --verdict REJECT \
  --notes "Missing acceptance-criteria coverage"

# Fail: blue sends file back to pending
gate advance T-1.1-auth-login-jwt.md --root . --stage blue --verdict FAIL \
  --notes "Scoped tests still failing"
```

The `advance` subcommand records the transition, appends the canonical section, updates the cache, and moves blue PASS tasks into `done/`. `gate` does **not** auto-escalate `attempts >= 3`; repeated REJECT/FAIL cycles are a PM decision point.

#### Sprint tracking

```bash
# Sprint progress dashboard
gate status --root .

# Machine-readable task state for scripts/dashboards
gate status --root . --json
```

From the JSON output: `done = count(status == "done")`, `remaining = count(status != "done")`, `total = len(tasks)`. Sprint complete means no task remains in `pending`, `red`, or `blue`, and `todo/` is empty.

#### Requirements traceability

```bash
# Per-task file/status/location view
gate status --root . --json
```

`gate status` gives you the live per-file state derived from disk. Correlate that JSON with `PRD.md`, `RTM.md`, and the task files themselves to answer requirement-coverage questions. If you need a relational traceability report, query `./pipeline.db` as a cache-backed convenience — never as the source of truth.

#### Health and analytics

```bash
# Operational dashboard
gate status --root .

# Detailed task-by-task state for scripts
gate status --root . --json
```

`gate status` is the primary dashboard for operational health. It exposes file-derived state and is the default source for dispatch decisions.

Detailed analytics still require raw SQLite against the cache because `gate` does not yet expose reject-rate, duration, or work-log drilldowns. Treat those queries as optional reporting over a rebuildable cache:

```bash
# Reject rate
sqlite3 ./pipeline.db "SELECT \
    printf('%.1f%%', 100.0 * sum(CASE WHEN verdict = 'REJECT' THEN 1 ELSE 0 END) \
        / count(*)) as reject_rate \
FROM work_log WHERE stage = 'red';"

# Average time per stage
sqlite3 ./pipeline.db "SELECT stage, count(*) as runs, \
    printf('%.0fs', avg(duration_seconds)) as avg_duration \
FROM work_log GROUP BY stage;"

# Work log for a specific task (full audit trail)
sqlite3 ./pipeline.db "SELECT stage, agent_model, verdict, duration_seconds, notes, created_at \
FROM work_log WHERE file = 'T-1.1-auth-login-jwt.md' ORDER BY id;"

# Tasks with most attempts (trouble spots)
sqlite3 ./pipeline.db "SELECT file, attempts, status FROM tasks \
WHERE attempts > 0 ORDER BY attempts DESC;"
```

### Dispatch pattern

```
You are the {role} agent. Read your instructions from {prompt_file}
then process the spec at {spec_file_path}
```

The PM never tells the agent what's in the spec. Never reads the spec itself. Never writes code or runs tests.

## Streaming Dispatch

**Never batch.** The pipeline is a stream.

1. Poll all active agents in parallel
2. For each completion: read verdict → run the `advance` subcommand → spawn next-stage agent
3. Any free slot → pull next pending → spawn SWE
4. Repeat

At steady state: a **mix** of SWE + red + blue running concurrently. All same type = batching regression.

### Slot management

Target 8 concurrent agents. When one finishes:

1. Process result → run the `advance` subcommand (blue PASS moves the file to `done/`)
2. File advanced? → spawn next-stage agent
3. Slot still free? → pull next pending for SWE
4. No pending left? → pipeline is draining

Never leave slots idle when work remains.

## Spec File Quality

A spec file is a **contract**. If it's ambiguous, SWE guesses. If it's wrong, SWE implements the wrong thing and red-team approves it.

Every spec should:

- Trace to a requirement from PRD.md (through RTM.md)
- Define acceptance criteria
- Specify verdicts when applicable (KEEP, REWRITE, REMOVE, MERGE, ADD)
- Prioritize recommendations (P0 = mandatory, P1–P2 = should-do)

## Prompt Design

Agent prompts must be:

- **Self-contained.** Agents are stateless. No context between runs.
- **Scoped testing only.** The exact command for running only relevant tests. Never the full suite.
- **Append-only output.** Each agent appends its section. This creates the audit trail and doubles as disaster recovery.

## Recovery

If the cache is lost, rebuild it from the filesystem:

```bash
# Rebuild pipeline.db from authoritative files on disk
gate sync --rebuild .
```

`gate sync --rebuild .` reparses `todo/` and `done/`, reconstructs cached task/work-log rows, and re-derives status/location from the canonical markdown sections. Manual `grep` + `sqlite3` reconstruction is no longer the recovery path.

The filesystem IS the backup. `done/` is authoritative for completed work. `todo/` file contents reveal in-progress state. `pipeline.db` is disposable.

## Health Signals

| Signal | Healthy | Investigate |
|--------|---------|-------------|
| Reject rate | < 5% | > 5% = specs are ambiguous or SWE prompt is weak |
| Reject rate | > 0% | 0% = red-team isn't catching anything — stronger model needed |
| Agent mix | 3–4 SWE, 2–3 red, 1–2 blue | All same type = batching regression |
| Stuck count | 0 | > 0 = structural problems, escalate to human |
| Slot utilization | 100% while work remains | Idle slots = orchestration bug |
| done/ file count | growing | stalled = pipeline blocked |

## Reference Templates

Ready-to-use templates in `references/`:

| File | What It Is |
|------|-----------|
| `references/scaffold/` | **Complete working pipeline** — copy and adapt |
| `references/scaffold/README.md` | Scaffold usage notes — rebuild `pipeline.db` with `gate sync --rebuild .` |
| `references/scaffold/PRD.md` | Sample requirements doc |
| `references/scaffold/RTM.md` | Sample traceability matrix |
| `references/scaffold/todo/` | Sample pending spec |
| `references/scaffold/done/` | Sample completed spec with full audit trail |
| `references/swe-prompt-template.md` | SWE agent prompt — replace `{scoped_test_command}` |
| `references/red-prompt-template.md` | Red-team audit prompt |
| `references/blue-prompt-template.md` | Blue-team verification prompt |
| `references/setup-template.md` | Pipeline initialization walkthrough |

## Stop Conditions

- You never read spec file contents. Agents do.
- You never write code or run tests. Agents do.
- You never modify spec files. Agents append their own sections.
- If you catch yourself doing any of the above, stop and dispatch an agent instead.
