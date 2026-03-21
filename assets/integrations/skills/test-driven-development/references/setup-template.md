# Pipeline Setup Template

Step-by-step instructions for initializing a new dispatch pipeline.

## 1. Directory Structure

```
{pipeline_root}/
├── PRD.md              # Requirements (FR-*, NFR-*)
├── RTM.md              # Traceability matrix (FR-* → TC-*)
├── todo/               # Backlog — pending spec files (inbox)
│   ├── T-1.1-feature-a.md
│   ├── T-1.2-feature-b.md
│   └── T-1.3-bug-fix-c.md
├── done/               # Archive — completed specs with audit trails
├── pipeline.db         # SQLite state tracker (gitignored)
├── swe-prompt.md       # SWE agent instructions (from template)
├── red-prompt.md       # Red-team agent instructions (from template)
└── blue-prompt.md      # Blue-team agent instructions (from template)
```

`PRD.md` defines what to build. `RTM.md` traces requirements to components and test cases. `todo/` is the inbox — each file is one backlog item (a T-* spec referencing FR-* from PRD.md). `done/` is the archive — completed files with their full audit trails. Both folders are committed. There is no TODO.md — `ls todo/` is the backlog.

## 2. Create Requirements + Backlog Files

```bash
mkdir -p todo done
```

Create `PRD.md` first, then write your spec files into `todo/`. The pipeline is file-authoritative, so the backlog is whatever markdown task files exist in `todo/`.

## 3. Initialize the Cache

```bash
gate sync --rebuild .
```

## 4. Write Agent Prompts

Copy the templates from this skill's `references/` directory and replace `{placeholders}`:

- `{scoped_test_command}` — the command that runs only the relevant tests for one spec file
- `{scoped_test_command_example}` — a concrete example of that command

The scoped test command is the single most important thing to get right. If agents can't run targeted tests, they'll either run the full suite (slow, noisy) or skip testing (dangerous).

## 5. Gitignore the DB

```bash
echo "pipeline.db" >> .gitignore
```

The cache is rebuildable operational state, not source code. The spec files with their accumulated audit trails are the persistent record.

## 6. Verify

```bash
gate status --root .
# Should show the same backlog represented by files in todo/ and done/
```

## 7. Start the Pipeline

Tell the PM orchestrator (or load the dispatch-pipeline skill) and provide:

- Path to the pipeline root
- Slot limit (default: 8)
- Agent types to use (SWE, red-team, blue-team)

The PM will use `gate` and start filling slots.

## Spec File Format

A spec file can be anything — the pipeline doesn't parse it. What matters is that the spec contains enough context for each agent to do its job without asking questions.

Good spec files include:

- **What to change** — specific files, functions, behaviors
- **Acceptance criteria** — what "done" looks like
- **Verdicts** (if applicable) — KEEP, REWRITE, REMOVE, MERGE, ADD
- **Recommendations** — prioritized (P0 = mandatory, P1–P2 = should-do)

The agents append their sections to the bottom of the spec file as they work, creating a linear audit trail per work item.
