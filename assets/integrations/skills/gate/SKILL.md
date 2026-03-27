---
name: gate
description: "The team's gated delivery process. Use this skill when the user asks to \"start a sprint\", \"run pipeline\", \"set up a pipeline\", \"dispatch agents\", \"coordinate parallel work\", \"run eval specs\", \"process specs through agents\", mentions \"gate.db\", \"eval_queue\", \"tasks table\", \"work_log\", \"pipeline\", \"sprint planning\", \"requirements traceability\", or wants to orchestrate subagents through a staged pipeline workflow. Also use when the user says \"gate\", \"SDD\", \"TDD\", \"GSD\", or references PRD/RTM/TODO traceability."
---

# Gate

Everything is a file. The database is a rebuildable cache.

A spec file is a unit of work. It lives in `todo/`. Agents read it, do the work for the current stage, and append their results. When the pipeline finishes with it, a final-stage `gate pass` moves it to `done/`. Markdown task files under `todo/` and `done/` are authoritative. `gate.db` is only a fast query cache rebuilt from disk whenever needed.

## Philosophy

The unix view: small files, each representing one task, flowing through a pipeline of transforms. The cache is `ps` — it gives you fast queryability into what's running and where. The files are the filesystem — they hold the actual content and accumulate history. `todo/` is the inbox. `done/` is the archive. Agents are processes.

- **Files are authoritative.** Task state lives in markdown files under `todo/` and `done/`.
- **`gate.db` is a rebuildable cache.** Use it for fast queries, but treat it as disposable; `gate sync --rebuild .` recreates it from disk at any time.
- **The `gate append`, `gate pass`, and `gate reject` subcommands enforce legal transitions** and append the canonical section for the active stage before updating the cache.
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

Only the PM adds/modifies requirements (`PRD.md`, `RTM.md`) or moves files to `done/`.

## Built-in Pipelines

Gate ships with three pipeline definitions and supports custom ones.

### SDD — Spec-Driven Development (default)

```
swe → red → blue → done
```

The current default. Stages: implement per spec → adversarial audit → verification.
Agents: SWE (implement), Red-team (audit), Blue-team (verify).

### TDD — Test-Driven Development

```
test → implement → verify → done
```

Classic Red/Green/Refactor: write failing test → implement to pass → refactor and verify.
Use `--pipeline tdd` or place a `pipeline.json` with TDD stages in your project root.

### GSD — Get Shit Done

```
research → plan → execute → verify → done
```

Research-first methodology: investigate → plan → execute → verify deliverables.
Based on the GSD framework (https://github.com/gsd-build/get-shit-done/).

### Custom Pipelines

Create a `pipeline.json` in your pipeline root:

```json
{
  "name": "my-pipeline",
  "stages": [
    { "name": "draft", "verdicts": [], "transitions": { "pass": "review" } },
    { "name": "review", "verdicts": ["pass", "reject"], "transitions": { "pass": "done", "reject": "pending" } }
  ]
}
```

Place it at `{pipeline_root}/pipeline.json`. Gate auto-discovers it.

## The Pipeline

```
todo/T-1.1-example.md ──▶ stage-1 ──▶ stage-2 ──▶ ... ──▶ final-stage ──▶ done/T-1.1-example.md
      ▲                     │                                   │
      │      REJECT/FAIL    │                 REJECT/FAIL       │
      └─────────────────────┴───────────────────────────────────┘
```

Each stage appends its section to the file. Rejections or failures return the task to the configured retry state — usually `pending`, but the exact transition comes from the active pipeline definition. The next agent attempt sees the rationale already captured in the file. On completion, the file moves from `todo/` to `done/` carrying the full audit trail.

The default SDD pipeline is one concrete example:

```
todo/T-1.1-auth-login-jwt.md ──▶ SWE ──▶ Red-team ──▶ Blue-team ──▶ done/T-1.1-auth-login-jwt.md
```

### Stage-to-agent mapping

Each pipeline stage maps to an agent role. The PM dispatches the stage's agent, the agent reads the spec plus prior appended sections, and the PM records the result with `gate append`, `gate pass`, or `gate reject`.

| Stage | Reads | Does | Appends |
|-------|-------|------|---------|
| Current stage agent | spec + all prior stage output relevant to this task | performs the stage's work | `## {Stage Name}` |
| Next stage agent | spec + prior stage sections | validates or advances from prior stage output | `## {Next Stage Name}` |

For SDD specifically:

| Agent | Reads | Does | Appends |
|-------|-------|------|---------|
| SWE | spec + any prior rejections/failures | implements, tests, commits | `## SWE Remediation` |
| Red-team | spec + SWE section | audits SWE against spec | `## Red-team Audit` |
| Blue-team | spec + SWE + red sections | runs scoped verification | `## Blue-team Verification` |

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
        CHECK(length(status) > 0),
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
    stage TEXT NOT NULL CHECK(length(stage) > 0),
    agent_model TEXT,             -- 'gpt-5.4', 'claude-sonnet-4.6'
    verdict TEXT,
    duration_seconds INTEGER,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

`requirements` and `traceability` mirror `PRD.md` and `RTM.md` on disk. They provide the relational backbone for cache-backed reporting, but the markdown files remain canonical.

`tasks` points to files. The `location` column tracks whether the file is in `todo/` or `done/`. The `requirement` FK connects every task back to the PRD.

`work_log` is append-only. Every agent action gets a row in the cache. This is the audit trail in queryable form — the file on disk has the same info in markdown, but the cache lets you ask questions without parsing markdown.

Stage names in `tasks.status` and `work_log.stage` are pipeline-dependent. In SDD you might see `pending`, `red`, `blue`, and `done`; in TDD you might see `pending`, `test`, `implement`, `verify`, and `done`. The schema only enforces "non-empty valid stage name here"; the active pipeline definition decides what those names mean.

The PM is the only writer. Agents never touch the cache directly; the PM uses the `append`, `pass`, `reject`, `status`, and `sync` subcommands as the canonical interface.

## Directory Layout

```
{pipeline_root}/
├── PRD.md                # Requirements (FR-*, NFR-*)
├── RTM.md                # Traceability matrix (FR-* → TC-*)
├── todo/                 # Backlog — pending spec files
├── done/                 # Archive — completed specs with audit trails
├── gate.db           # Rebuildable cache/index (gitignored)
├── pipeline.json         # Optional custom pipeline definition
├── swe-prompt.md         # SDD example prompt file
├── red-prompt.md         # SDD example prompt file
└── blue-prompt.md        # SDD example prompt file
```

`PRD.md` and `RTM.md` are the contracts — what to build and how to trace it. `todo/` and `done/` are the work. All four are committed. `gate.db` is a rebuildable cache, gitignored.

Sprint planning = write spec files into `todo/`, each referencing FR-* from PRD.md. Sprint progress = `ls todo/ | wc -l` remaining, `ls done/ | wc -l` complete. Sprint close = `todo/` is empty.

If you stay on a built-in pipeline, you may not need `pipeline.json`. If you want custom stages or transitions, add one at the pipeline root.

## Setup

A working reference scaffold is in `references/scaffold/` — a ready-to-copy pipeline root with a seeded cache, sample PRD/RTM, one todo item, and one done item. The scaffold content is SDD-shaped by default, but the file-authoritative structure works for any pipeline.

```bash
# Copy scaffold to your pipeline root
cp -r references/scaffold/ /path/to/pipeline/

# Or start from scratch:
mkdir -p todo done
# Create PRD.md, RTM.md (see scaffold for examples)

# Optional: choose a built-in pipeline explicitly
gate status --root . --pipeline tdd

# Optional: define your own pipeline stages/transitions
# by creating pipeline.json in the pipeline root

# Rebuild gate.db from the authoritative files on disk
gate sync --rebuild .

# Write stage prompts (copy SDD examples from references/, or author your own)
echo "gate.db" >> .gitignore
```

### Gate binary location

`gate` is built from `crates/maenifold-gate`. Put the binary on your `PATH` to use the short `gate ...` form in prompts/scripts. During development, use `cargo run -p maenifold-gate -- <subcommand>`. The release binary is `target/release/gate`.

## The PM Dispatch Loop

Five things, forever:

1. **Query** `gate status` for files at each status
2. **Dispatch** the matching stage agent with the file path
3. **Read** the agent's verdict or completion note
4. **Record the result** with `gate append`, `gate pass`, or `gate reject`; a final-stage `gate pass` moves the file to `done/`
5. **Backfill** any freed slot immediately

### Queries

#### Dispatch operations

```bash
# Dashboard
gate status --root .

# Machine-readable queue snapshot
gate status --root . --json

# Built-in pipeline selection when needed
gate status --root . --pipeline tdd

# Next file for a given stage:
# filter the JSON array for the first object whose status matches
# a dispatchable stage in the active pipeline
gate status --root . --json

# Append: current stage completed and recorded
gate append T-1.1 --root . --agent implement \
  --notes "Completed this stage and recorded scoped results"

# Pass: advance to the next configured stage
gate pass T-1.1 --root . \
  --notes "Current stage passed; advancing per active pipeline"

# Final-stage pass: moves file to done/
gate pass T-1.1 --root . \
  --notes "Final verification passed"

# Reject or fail: return to the pipeline's configured retry state
gate reject T-1.1 --root . \
  --notes "Needs revision before re-entering pipeline"
```

These commands record the transition, append the canonical section for the active stage, update the cache, and a final-stage `gate pass` moves tasks into `done/`. `gate` does **not** auto-escalate `attempts >= 3`; repeated reject/fail cycles are a PM decision point.

#### Sprint tracking

```bash
# Sprint progress dashboard
gate status --root .

# Machine-readable task state for scripts/dashboards
gate status --root . --json
```

From the JSON output: `done = count(status == "done")`, `remaining = count(status != "done")`, `total = len(tasks)`. Sprint complete means no task remains in an active stage and `todo/` is empty.

#### Requirements traceability

```bash
# Per-task file/status/location view
gate status --root . --json
```

`gate status` gives you the live per-file state derived from disk. Correlate that JSON with `PRD.md`, `RTM.md`, and the task files themselves to answer requirement-coverage questions. If you need a relational traceability report, query `./gate.db` as a cache-backed convenience — never as the source of truth.

#### Health and analytics

```bash
# Operational dashboard
gate status --root .

# Detailed task-by-task state for scripts
gate status --root . --json
```

`gate status` is the primary dashboard for operational health. It exposes file-derived state and is the default source for dispatch decisions.

Detailed analytics still require raw SQLite against the cache because `gate` does not yet expose reject-rate, duration, or work-log drilldowns. Treat those queries as optional reporting over a rebuildable cache. Replace the example stage names below with the names from your active pipeline:

```bash
# Reject rate for an audit stage (SDD example)
sqlite3 ./gate.db "SELECT \
    printf('%.1f%%', 100.0 * sum(CASE WHEN verdict = 'REJECT' THEN 1 ELSE 0 END) \
        / count(*)) as reject_rate \
FROM work_log WHERE stage = 'red';"

# Average time per stage
sqlite3 ./gate.db "SELECT stage, count(*) as runs, \
    printf('%.0fs', avg(duration_seconds)) as avg_duration \
FROM work_log GROUP BY stage;"

# Work log for a specific task (full audit trail)
sqlite3 ./gate.db "SELECT stage, agent_model, verdict, duration_seconds, notes, created_at \
FROM work_log WHERE file = 'T-1.1-auth-login-jwt.md' ORDER BY id;"

# Tasks with most attempts (trouble spots)
sqlite3 ./gate.db "SELECT file, attempts, status FROM tasks \
WHERE attempts > 0 ORDER BY attempts DESC;"
```

### Dispatch pattern

```
You are the {stage} agent. Read your instructions from {prompt_file}
then process the spec at {spec_file_path}
```

The PM should not restate the spec in the dispatch prompt. The stage agent reads the spec directly, then returns a verdict or completion note that the PM records.

## Streaming Dispatch

**Never batch.** The pipeline is a stream.

1. Poll all active agents in parallel
2. For each completion: read verdict → run `gate append`, `gate pass`, or `gate reject` → spawn the next-stage agent if the pipeline advances
3. Any free slot → pull the next ready task for the earliest dispatchable stage
4. Repeat

At steady state, a healthy pipeline runs a **mix** of stages concurrently across different specs. All agents doing the same stage at once is usually a batching regression.

### Fleet mode integration

When `/fleet` mode is available (Copilot CLI), the PM should leverage it for parallel subagent dispatch. Fleet provides native subagent spawning which is faster and more resource-efficient than sequential `task` tool calls.

**Detection:** If the system prompt contains fleet mode instructions (e.g. "You are now in fleet mode" or mentions dispatching sub-agents via the `task` tool in parallel), use fleet-style parallel dispatch.

**How to use fleet with gate:**

1. **Dispatch is parallel, gates are serial.** Fleet parallelizes dispatch across *independent* specs. Within a single spec, stages still flow according to the active pipeline definition.
2. **Maximize heterogeneous concurrency.** Use fleet to dispatch a mix of stage agents simultaneously — each working on a *different* spec at its current stage.
3. **Respect stage dependencies.** A later stage for spec `T-1` cannot start until the prior stage for `T-1` completes. But `T-1` at one stage and `T-2` at another can run in parallel via fleet.
4. **Gate commands remain the PM's responsibility.** Fleet subagents do not run `gate append/pass/reject`. The PM reads each subagent's result, then runs the appropriate `gate` command before dispatching the next stage.

**Fleet dispatch pattern:**

```
# Instead of dispatching one agent at a time:
task(stage-a, T-1)  →  wait  →  task(stage-b, T-1)  →  wait  →  ...

# Dispatch all independent work in one fleet turn:
task(stage-a, T-1) + task(stage-b, T-2) + task(stage-c, T-3)  →  process results  →  next wave
```

For SDD, that heterogeneous mix is often `task(swe, T-1) + task(red, T-2) + task(blue, T-3)`.

**When fleet is NOT available:** Fall back to the standard `task` tool dispatch below. The pipeline logic is identical; only the dispatch mechanism changes.

### Slot management

Target 8 concurrent agents. When one finishes:

1. Process result → run `gate append`, `gate pass`, or `gate reject` (a final-stage `gate pass` moves the file to `done/`)
2. File moved forward? → spawn the next-stage agent
3. Slot still free? → pull the next ready task for the earliest dispatchable stage
4. No ready work left? → pipeline is draining

Never leave slots idle when work remains.

## Spec File Quality

A spec file is a **contract**. If it's ambiguous, agents guess. If it's wrong, later stages may approve the wrong thing.

Every spec should:

- Trace to a requirement from `PRD.md` (through `RTM.md`)
- Define acceptance criteria
- Define stage-specific exit criteria or verdicts where applicable
- Prioritize mandatory work vs optional improvements

## Prompt Design

Agent prompts must be:

- **Self-contained.** Agents are stateless. No context between runs.
- **Scoped.** Each stage gets only the commands and authority it needs.
- **Append-only output.** Each agent appends its section. This creates the audit trail and doubles as disaster recovery.

The prompt templates in `references/` are SDD-specific (`swe`, `red`, `blue`). If you use TDD, GSD, or a custom pipeline, author equivalent prompts for those stages.

## Recovery

If the cache is lost, rebuild it from the filesystem:

```bash
# Rebuild gate.db from authoritative files on disk
gate sync --rebuild .
```

`gate sync --rebuild .` reparses `todo/` and `done/`, reconstructs cached task/work-log rows, and re-derives status/location from the canonical markdown sections. Manual `grep` + `sqlite3` reconstruction is no longer the recovery path.

The filesystem IS the backup. `done/` is authoritative for completed work. `todo/` file contents reveal in-progress state. `gate.db` is disposable.

## Health Signals

| Signal | Healthy | Investigate |
|--------|---------|-------------|
| Reject/fail rate | low but non-zero | high = specs or prompts are weak; zero on audit stages can mean the gate is not catching anything |
| Agent mix | multiple stages active at once | all same type = batching regression |
| Stuck count | 0 | > 0 = structural problems, escalate to human |
| Slot utilization | 100% while work remains | idle slots = orchestration bug |
| `done/` file count | growing | stalled = pipeline blocked |

## Reference Templates

Ready-to-use templates in `references/`:

| File | What It Is |
|------|-----------|
| `references/scaffold/` | **Complete working pipeline scaffold** — copy and adapt |
| `references/scaffold/README.md` | Scaffold usage notes — rebuild `gate.db` with `gate sync --rebuild .` |
| `references/scaffold/PRD.md` | Sample requirements doc |
| `references/scaffold/RTM.md` | Sample traceability matrix |
| `references/scaffold/todo/` | Sample pending spec |
| `references/scaffold/done/` | Sample completed spec with full audit trail |
| `references/swe-prompt-template.md` | SDD SWE agent prompt — replace `{scoped_test_command}` |
| `references/red-prompt-template.md` | SDD Red-team audit prompt |
| `references/blue-prompt-template.md` | SDD Blue-team verification prompt |
| `references/setup-template.md` | Pipeline initialization walkthrough |

## Stop Conditions

- You do not rewrite spec contents into dispatch prompts. Stage agents read the spec directly.
- You do not write code or run tests if the active pipeline expects stage agents to do that work.
- You do not manually edit stage-owned audit/remediation sections after the agent has written them.
- If you catch yourself doing any of the above, stop and dispatch the appropriate stage agent instead.
