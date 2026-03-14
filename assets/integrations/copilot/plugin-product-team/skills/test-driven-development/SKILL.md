---
name: test-driven-development
description: The team's test-driven development process. Use this skill when the user asks to "start a sprint", "run TDD", "set up a pipeline", "dispatch agents", "coordinate parallel work", "run eval specs", "process specs through agents", mentions "pipeline.db", "eval_queue", "tasks table", "work_log", "SWE/red/blue pipeline", "sprint planning", "requirements traceability", or wants to orchestrate subagents through the implement → audit → verify workflow. Also use when the user says "TDD", "test-driven", or references PRD/RTM/TODO traceability.
---

# Test-Driven Development Process

Everything is a file. The database is state.

A spec file is a unit of work. It lives in `todo/`. Agents read it, do the work, append their results. When the pipeline finishes with it, it moves to `done/`. The SQLite database tracks where each file is in the pipeline. That's the whole system.

## Philosophy

The unix view: small files, each representing one task, flowing through a pipeline of transforms. The DB is `ps` — it tells you what's running and where. The files are the filesystem — they hold the actual content and accumulate history. `todo/` is the inbox. `done/` is the archive. Agents are processes.

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
todo/spec.md ──▶ SWE ──▶ Red-team ──▶ Blue-team ──▶ done/spec.md
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

### The database

Four tables. Files on disk are the artifacts. DB is the index + state machine.

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
    file TEXT PRIMARY KEY,        -- 'auth-login-jwt.md'
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

`requirements` and `traceability` mirror the PRD.md and RTM.md on disk. They're loaded at setup and provide the relational backbone — "which requirements have no tasks?" is a JOIN, not a grep.

`tasks` points to files. The `location` column tracks whether the file is in `todo/` or `done/`. The `requirement` FK connects every task back to the PRD.

`work_log` is append-only. Every agent action gets a row. This is the audit trail in queryable form — the file on disk has the same info in markdown, but the DB lets you ask "what's our average red-team reject rate?" without parsing markdown.

The PM is the only writer. Agents never touch it.

## Directory Layout

```
{pipeline_root}/
├── PRD.md                # Requirements (FR-*, NFR-*)
├── RTM.md                # Traceability matrix (FR-* → TC-*)
├── todo/                 # Backlog — pending spec files
├── done/                 # Archive — completed specs with audit trails
├── pipeline.db           # State tracker (gitignored)
├── swe-prompt.md         # SWE agent instructions
├── red-prompt.md         # Red-team agent instructions
└── blue-prompt.md        # Blue-team agent instructions
```

`PRD.md` and `RTM.md` are the contracts — what to build and how to trace it. `todo/` and `done/` are the work. All four are committed. `pipeline.db` is ephemeral state, gitignored.

Sprint planning = write spec files into `todo/`, each referencing FR-* from PRD.md. Sprint progress = `ls todo/ | wc -l` remaining, `ls done/ | wc -l` complete. Sprint close = `todo/` is empty.

## Setup

A working reference scaffold is in `references/scaffold/` — a ready-to-copy pipeline root with a seeded DB, sample PRD/RTM, one todo item, and one done item. Copy it, adapt it, go.

```bash
# Copy scaffold to your pipeline root
cp -r references/scaffold/ /path/to/pipeline/

# Or start from scratch:
mkdir -p todo done
# Create PRD.md, RTM.md (see scaffold for examples)

# Create DB (full schema in references/scaffold/pipeline.db)
sqlite3 pipeline.db < references/schema.sql

# Load requirements from PRD.md
sqlite3 pipeline.db "INSERT INTO requirements (id, type, description) VALUES
    ('FR-1', 'FR', 'System SHALL provide user authentication via JWT tokens');"

# Load traceability from RTM.md
sqlite3 pipeline.db "INSERT INTO traceability (test_case, requirement, component, test_type) VALUES
    ('TC-1', 'FR-1', 'src/auth/', 'integration');"

# Load tasks (one per spec file in todo/)
for f in todo/*.md; do
    sqlite3 pipeline.db "INSERT INTO tasks (file, title, requirement) VALUES
        ('$(basename $f)', 'TODO: set title', 'TODO: set requirement');"
done

# Write agent prompts (copy from references/, adapt test commands)
echo "pipeline.db" >> .gitignore
```

## The PM Dispatch Loop

Five things, forever:

1. **Query** the DB for files at each status
2. **Dispatch** the matching agent with the file path
3. **Read** the agent's verdict
4. **Advance or reject** in the DB; on done, `mv todo/{file} done/{file}`
5. **Backfill** any freed slot immediately

### Queries

#### Dispatch operations

```sql
-- Dashboard
SELECT status, count(*) FROM tasks GROUP BY status;

-- Next file for SWE
SELECT file FROM tasks WHERE status = 'pending' ORDER BY file LIMIT 1;

-- Advance: SWE done → red
INSERT INTO work_log (file, stage, agent_model, duration_seconds, notes)
    VALUES (?, 'swe', ?, ?, ?);
UPDATE tasks SET status = 'red', updated_at = datetime('now') WHERE file = ?;

-- Advance: red passed → blue
INSERT INTO work_log (file, stage, agent_model, verdict, duration_seconds, notes)
    VALUES (?, 'red', ?, 'PASS', ?, ?);
UPDATE tasks SET status = 'blue', updated_at = datetime('now') WHERE file = ?;

-- Advance: blue passed → done (then mv todo/{file} done/{file})
INSERT INTO work_log (file, stage, agent_model, verdict, duration_seconds, notes)
    VALUES (?, 'blue', ?, 'PASS', ?, ?);
UPDATE tasks SET status = 'done', location = 'done', updated_at = datetime('now') WHERE file = ?;

-- Reject: back to pending
INSERT INTO work_log (file, stage, agent_model, verdict, duration_seconds, notes)
    VALUES (?, ?, ?, 'REJECT', ?, ?);
UPDATE tasks SET status = 'pending', attempts = attempts + 1,
    updated_at = datetime('now') WHERE file = ?;

-- Escalate (3 strikes)
UPDATE tasks SET status = 'stuck'
    WHERE attempts >= 3 AND status = 'pending';
```

#### Sprint tracking

```sql
-- Sprint progress
SELECT 
    (SELECT count(*) FROM tasks WHERE status = 'done') as done,
    (SELECT count(*) FROM tasks WHERE status != 'done') as remaining,
    (SELECT count(*) FROM tasks) as total,
    printf('%.0f%%', 100.0 * (SELECT count(*) FROM tasks WHERE status = 'done') 
        / (SELECT count(*) FROM tasks)) as progress;

-- Sprint complete?
SELECT CASE WHEN count(*) = 0 THEN 'YES' ELSE 'NO' END as sprint_complete
FROM tasks WHERE status != 'done';
```

#### Requirements traceability

```sql
-- Requirement coverage (which requirements have tasks, which are done)
SELECT 
    r.id, r.description,
    count(t.file) as total_tasks,
    sum(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) as done,
    sum(CASE WHEN t.status != 'done' THEN 1 ELSE 0 END) as remaining
FROM requirements r
LEFT JOIN tasks t ON t.requirement = r.id
GROUP BY r.id;

-- Requirements with no tasks (uncovered)
SELECT r.id, r.description
FROM requirements r
LEFT JOIN tasks t ON t.requirement = r.id
WHERE t.file IS NULL;

-- Full traceability chain
SELECT r.id as req, tr.test_case, tr.component, t.file, t.status
FROM requirements r
JOIN traceability tr ON tr.requirement = r.id
LEFT JOIN tasks t ON t.requirement = r.id
ORDER BY r.id, tr.test_case;
```

#### Health and analytics

```sql
-- Reject rate
SELECT 
    printf('%.1f%%', 100.0 * sum(CASE WHEN verdict = 'REJECT' THEN 1 ELSE 0 END) 
        / count(*)) as reject_rate
FROM work_log WHERE stage = 'red';

-- Average time per stage
SELECT stage, count(*) as runs,
    printf('%.0fs', avg(duration_seconds)) as avg_duration
FROM work_log GROUP BY stage;

-- Work log for a specific task (full audit trail)
SELECT stage, agent_model, verdict, duration_seconds, notes, created_at
FROM work_log WHERE file = ? ORDER BY id;

-- Tasks with most attempts (trouble spots)
SELECT file, attempts, status FROM tasks WHERE attempts > 0 ORDER BY attempts DESC;
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
2. For each completion: read verdict → update DB → spawn next-stage agent
3. Any free slot → pull next pending → spawn SWE
4. Repeat

At steady state: a **mix** of SWE + red + blue running concurrently. All same type = batching regression.

### Slot management

Target 8 concurrent agents. When one finishes:

1. Process result → update DB (→ `mv` to `done/` if complete)
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

If the DB is lost, reconstruct from the filesystem:

```bash
# Recreate schema
sqlite3 pipeline.db < references/schema.sql
# (or copy from references/scaffold/pipeline.db and clear data)

# Load requirements/traceability from PRD.md and RTM.md
# (parse manually or script it — these change rarely)

# Files already in done/ → status = done
for f in done/*.md; do
    sqlite3 pipeline.db "INSERT INTO tasks (file, title, requirement, status, location) VALUES
        ('$(basename $f)', '', 'TODO', 'done', 'done');"
done

# Files still in todo/ — check audit trail headers
# Blue-team section present → done (mv to done/ too)
grep -rl "Blue-team Verification" todo/ | while read f; do
    mv "$f" done/
    sqlite3 pipeline.db "INSERT INTO tasks (file, title, requirement, status, location) VALUES
        ('$(basename "$f")', '', 'TODO', 'done', 'done');"
done

# Red-team section but no blue → status = blue
grep -rl "Red-team Audit" todo/ | xargs grep -rL "Blue-team Verification" | while read f; do
    sqlite3 pipeline.db "INSERT INTO tasks (file, title, requirement, status, location) VALUES
        ('$(basename "$f")', '', 'TODO', 'blue', 'todo');"
done

# SWE section but no red → status = red
grep -rl "SWE Remediation" todo/ | xargs grep -rL "Red-team Audit" | while read f; do
    sqlite3 pipeline.db "INSERT INTO tasks (file, title, requirement, status, location) VALUES
        ('$(basename "$f")', '', 'TODO', 'red', 'todo');"
done

# Everything else → status = pending
for f in todo/*.md; do
    sqlite3 pipeline.db "INSERT OR IGNORE INTO tasks (file, title, requirement, status, location) VALUES
        ('$(basename $f)', '', 'TODO', 'pending', 'todo');"
done

# Rebuild work_log from file sections (grep for agent headers in done/)
# Then backfill requirement FKs from PRD.md
```

The filesystem IS the backup. `done/` is authoritative for completed work. `todo/` file contents reveal in-progress state.

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
| `references/scaffold/pipeline.db` | Seeded DB with 1 todo, 1 done, full work log |
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
