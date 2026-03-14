# Multi-Agent Pipeline Orchestration

Everything is a file. The database is state.

A spec file is a unit of work. It lives in `todo/`. Agents read it, do the work, append their results. When the pipeline finishes with it, it moves to `done/`. The SQLite database tracks where each file is in the pipeline. That's the whole system.

See the `test-driven-development` skill for the full process including requirements hierarchy, sprint lifecycle, and agent prompt templates.

## How It Works

```
┌─────────┐     ┌──────────┐     ┌───────────┐     ┌──────────┐
│ pending │────▶│   SWE    │────▶│ Red-team  │────▶│Blue-team │────▶ done
│         │     │implements│     │  audits   │     │ verifies │
└─────────┘     └──────────┘     └───────────┘     └────┬─────┘
     ▲                                │                  │
     │              REJECT            │           REJECT │
     └────────────────────────────────┘                  │
     └───────────────────────────────────────────────────┘
```

Every work item is a spec file — a markdown document that defines what "done" looks like. Spec files live in a `todo/` folder and never move. A SQLite database tracks each file's status through the pipeline. Agents append their results to the spec file as they work, creating a linear audit trail.

## Setting Up

### 1. Write spec files

Each spec file describes one unit of work. It should contain enough context for an agent to complete the work without asking questions — requirements, acceptance criteria, what to change, how to verify.

Put them in a `todo/` folder:

```
project/
├── todo/
│   ├── add-auth-middleware.md
│   ├── fix-pagination-bug.md
│   └── refactor-db-pool.md
├── pipeline.db
├── swe-prompt.md
├── red-prompt.md
└── blue-prompt.md
```

### 2. Create the pipeline database

```sql
CREATE TABLE eval_queue (
    file TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','red','blue','done','stuck')),
    attempts INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Load the spec files:

```bash
for f in todo/*.md; do
    sqlite3 pipeline.db "INSERT INTO eval_queue (file) VALUES ('$(basename $f)');"
done
```

### 3. Write agent prompts

Three prompt files, one per agent role. Each must be self-contained — agents are stateless and know nothing about prior runs. The prompts must include:

- **What to read**: the spec file path (the PM provides this at dispatch time)
- **How to build/test**: the exact commands for this project (e.g., `cargo test --test integration_tests -p my-crate -- {module}`, or `npm test -- --grep {pattern}`)
- **What to output**: a clear PASS or REJECT verdict, plus rationale
- **Where to write results**: append a section to the spec file

The PM never needs to know these details. It just points each agent at its prompt file and the spec file to process.

## Running the Pipeline

### The PM's job

The PM orchestrator does exactly five things, in a loop:

1. **Query the DB** for files at each status
2. **Dispatch** the matching agent type with the spec file path
3. **Read** the agent's verdict (PASS / REJECT)
4. **Advance or reject** the file in the DB
5. **Backfill** any freed agent slot immediately

The PM never reads spec file contents. It never writes code. It never runs tests. It is a state machine operator.

### Dispatch queries

```sql
-- Dashboard
SELECT status, count(*) FROM eval_queue GROUP BY status;

-- Next file for SWE
SELECT file FROM eval_queue WHERE status = 'pending' ORDER BY file LIMIT 1;

-- Advance: SWE done → red-team
UPDATE eval_queue SET status = 'red', updated_at = datetime('now') WHERE file = ?;

-- Advance: red-team passed → blue-team
UPDATE eval_queue SET status = 'blue', updated_at = datetime('now') WHERE file = ?;

-- Advance: blue-team passed → done
UPDATE eval_queue SET status = 'done', updated_at = datetime('now') WHERE file = ?;

-- Reject: back to pending
UPDATE eval_queue SET status = 'pending', attempts = attempts + 1,
    updated_at = datetime('now') WHERE file = ?;

-- Escalate after repeated failures
UPDATE eval_queue SET status = 'stuck'
    WHERE attempts >= 3 AND status = 'pending';
```

### Streaming dispatch (critical)

**Never batch.** Never wait for all running agents to finish before advancing any of them. The pipeline is a stream, not a series of waves.

The correct loop:

1. Poll all active agents in parallel
2. For each completion:
   - Read its verdict
   - Update the DB (advance or reject)
   - Immediately spawn the next-stage agent for that file
3. If any slot is now free, pull the next `pending` file and spawn a SWE
4. Repeat

At steady state you should see a **mix** of all three agent types running simultaneously. If all running agents are the same type, the orchestrator has degenerated into batching.

### Slot management

Pick a slot limit (e.g., 8). When an agent finishes:

1. Process its result → update DB
2. File advanced to next stage? → spawn that stage's agent (fills the slot)
3. Slot still free? → pull next `pending` file for SWE
4. No pending files left? → slot stays empty; pipeline is draining

Never exceed the limit. Never leave slots idle when work remains.

## The Three Agent Roles

### SWE agent

- Picks up `pending` files
- Reads the spec, implements changes, runs targeted tests, commits
- Appends a `## SWE Remediation` section to the spec file documenting what it did
- PM advances to `red` on completion

### Red-team agent

- Picks up `red` files
- Audits the SWE's changes against the spec — did the agent do what the spec asked?
- **Read-only** — never modifies code
- Appends a `## Red-team Audit` section with PASS or REJECT
- REJECT sends the file back to `pending`; the SWE will see the rejection rationale on the next attempt

### Blue-team agent

- Picks up `blue` files
- Runs the targeted tests to verify the build is green
- Appends a `## Blue-team Verification` section with PASS or FAIL
- PASS → done. FAIL → back to `pending`

## Prompt Design

The quality of the pipeline depends almost entirely on two things: spec file quality and prompt quality.

### Spec files

A spec file is a contract. If it's ambiguous, the SWE will guess. If it's wrong, the SWE will implement the wrong thing and the red-team will approve it. The spec should be grounded in real requirements, not vibes.

### Agent prompts

- **Self-contained.** Every prompt must work for a brand new, stateless agent. No assumed context.
- **Scoped testing.** The prompt must include the exact command for running only the relevant tests. Agents must never run a full test suite — it wastes time and produces noise.
- **Append-only output.** Each agent appends its section to the spec file. This creates the audit trail: SWE → red → blue. This trail is also the disaster recovery mechanism (see below).

## Concurrency Model

The PM is the **single writer** to the pipeline DB. Agents never touch it. Agents communicate through:

1. Their return value to the PM (PASS/REJECT verdict)
2. The section they append to the spec file (detailed rationale)

This means zero locking, zero race conditions, zero coordination between agents. The PM serializes all state transitions.

## Recovery

If the DB gets corrupted or lost, reconstruct it from the spec files. Each agent appends a section with a known header:

```bash
# Files that completed all 3 stages → done
grep -rl "Blue-team Verification" todo/ | wc -l

# Files with red-team audit but no blue-team → status = blue
grep -rl "Red-team Audit" todo/ | xargs grep -rL "Blue-team Verification"

# Files with SWE work but no red-team → status = red
grep -rl "SWE Remediation" todo/ | xargs grep -rL "Red-team Audit"

# Everything else → status = pending
```

This is why the append-only audit trail matters — it's the backup state store. Agents must always append their section, even on REJECT.

## Health Signals

| Signal | Healthy | Investigate |
|--------|---------|-------------|
| Reject rate | < 5% | > 5% means specs are ambiguous or SWE prompt is weak |
| Reject rate | > 0% | 0% means red-team isn't catching anything — use a stronger model |
| Agent mix | 3–4 SWE, 2–3 red, 1–2 blue | All same type = batching regression |
| Stuck count | 0 | > 0 means structural problems the pipeline can't self-correct |
| Slot utilization | 100% while work remains | Idle slots = orchestration bug |

## Gitignore the DB

`pipeline.db` is operational state, not source code. The spec files (with their accumulated audit trails) are the persistent record that gets committed.

## Agent Prompt Templates

Each agent type needs a prompt file. The PM passes the prompt file path and the spec file path when dispatching. Below are templates — adapt the build/test commands for your project.

### SWE Prompt (`swe-prompt.md`)

```markdown
# SWE — Spec Implementation

You are given a specific spec file to implement. The file path is provided
in your task assignment.

## Do

1. Read the spec file at the given path.
2. Note the source file(s), verdicts table, and recommendations.
3. If prior red-team/blue-team rejection sections exist at the bottom,
   fix those issues first.
4. Implement the verdicts:
   - **KEEP**: don't touch it.
   - **REWRITE**: rewrite per the detailed findings.
   - **REMOVE**: delete it.
   - **MERGE**: collapse identified items into one, delete originals.
   - **ADD**: create the new item per the spec.
5. Apply recommendations: P0 mandatory, P1–P2 should-do, P3+ only if small.
6. Verify your changes compile and pass with **only** the scoped test command:
   <!-- Replace with your project's test command pattern -->
   {scoped_test_command}
7. Append your results to the spec file:

   ---
   ## SWE Remediation
   - REWRITE item_X: strengthened per P0
   - REMOVE item_Y: confirmed subsumed
   - Commit: {sha}

8. Commit your code changes referencing the spec file name.

## Don't

- Run the full test suite. Only the scoped command above.
- Change items marked KEEP (beyond what rejection feedback requires).
- Add work beyond what the spec recommends.
- Move any files. The PM manages pipeline state.
```

### Red-team Prompt (`red-prompt.md`)

```markdown
# Red-team — Spec Audit

You are given a specific spec file to audit. The file path is provided
in your task assignment. The file contains the original spec plus an
appended SWE remediation section.

## Do

1. Read the spec file at the given path.
2. Read the original spec (verdicts table, findings, recommendations).
3. Read the SWE remediation section at the bottom.
4. Audit each change against the spec:
   - Did REWRITE items get the changes the spec specified?
   - Did REMOVE items actually get deleted (and were they truly subsumed)?
   - Did MERGE items cover all original cases?
   - Was P0 addressed? P1–P2?
   - Were any unrelated changes introduced?
5. Append your findings to the spec file:

   **On pass:**
   ---
   ## Red-team Audit
   - ✅ REWRITE item_X: matches spec
   - ✅ REMOVE item_Y: confirmed no coverage lost
   - Verdict: PASS

   **On reject:**
   ---
   ## Red-team Audit
   - ✅ REWRITE item_X: matches spec
   - ❌ MERGE item_Z: missing edge case from spec
   - Verdict: REJECT

6. Report your verdict (PASS or REJECT) clearly in your response.

## Don't

- Modify any source code. You audit only.
- Reject for style or preferences. The spec is the contract — audit
  against it, not your opinions.
- Move any files. The PM manages pipeline state.
```

### Blue-team Prompt (`blue-prompt.md`)

```markdown
# Blue-team — Verification

You are given a specific spec file to verify. The file path is provided
in your task assignment. The file contains the spec, SWE remediation,
and a red-team PASS.

## Do

1. Read the spec file at the given path to identify the target.
2. Run **only** the scoped test command:
   <!-- Replace with your project's test command pattern -->
   {scoped_test_command}
3. Append your results to the spec file:

   **On pass:**
   ---
   ## Blue-team Verification
   - `{test_command}`: N passed, 0 failed
   - Verdict: PASS

   **On fail:**
   ---
   ## Blue-team Verification
   - `{test_command}`: N passed, M failed
   - Failures: {list failures and errors}
   - Verdict: FAIL

4. Report your verdict (PASS or FAIL) clearly in your response.

## Don't

- Run the full test suite. Only the scoped command above.
- Modify any source code. You verify only.
- Move any files. The PM manages pipeline state.
```

### PM Dispatch Pattern

The PM doesn't need its own prompt file — it's the orchestrator running the loop. For each dispatch:

```
You are the {role} agent. Read your instructions from {prompt_file}
then process the spec at {spec_file_path}
```

The PM never tells the agent what's in the spec. The agent reads its own prompt, reads the spec, does the work, reports back.
