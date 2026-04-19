# RALPH Iteration Protocol

You are running a single autonomous improvement iteration for `maenifold-rs`.

## Bootstrap (mandatory, every iteration)

### 1. Load skills

Load these skills before doing anything else:
- `maenifold` — knowledge graph, memory, reasoning, workflows, pipelines
- `gate` — task backlog, pass/fail, process governance

### 2. Initialize gate

Sync the gate cache so status is current:

**MCP (preferred):** Use the gate MCP `sync` tool with root `.`
**CLI fallback:** `gate sync --root .`

### 3. Check gate status

Read the backlog to see what's pending:

**MCP (preferred):** Use the gate MCP `status` tool with root `.`
**CLI fallback:** `gate status --root .`

### 4. Read project conventions

Read `AGENTS.md` at repo root for project conventions and constitutional rules.

## Mission

Incrementally improve the codebase while preserving build and test stability.
Gate task files in `todo/` are the source of truth for requirements and progress.

## Knowledge grounding (mandatory)

Before making any change, search the knowledge graph for related prior work:

**MCP (preferred):**
- `SearchMemories` — find related notes
- `BuildContext` — traverse concept neighbors
- `SequentialThinking` — reason through complex tradeoffs

**CLI fallback:**
- `maenifold search "<query>"`
- `maenifold buildcontext "<concept>"`
- `maenifold think "<topic>"`

If the graph has relevant prior work, read it. Do not duplicate effort or contradict prior decisions without justification.

## Selection policy

- Pick the next pending gate task from status.
- Read the selected task file in `todo/` for title, description, and acceptance criteria.
- One improvement per iteration.
- If `RALPH_TARGET_CRATE` environment variable is set and non-empty, prioritize work in that crate.

## Change-size limits

- One crate at a time.
- Max 12 files changed.
- Max ~600 LOC net diff.
- Prefer simplification and deletion over adding new complexity.

## Execution protocol

1. Implement the selected pending gate task (`T-200.X`).
2. Run `cargo build --release -p <crate>` for the affected crate.
3. Run `cargo test -p <crate>` for the affected crate.
4. On failure, iterate fix attempts up to 3 times.
5. If still failing after 3 attempts:
   - Revert or abandon the partial change safely.
   - Record failure via gate:
     - **MCP (preferred):** Use gate MCP `fail` tool — taskId `T-200.X`, agent `ralph`, notes `<what went wrong>`
     - **CLI fallback:** `gate fail T-200.X --root . --agent ralph --notes '<what went wrong>'`
   - Append the failure pattern and lessons to `ralph/state/learnings.md`.
   - Move to next target on next iteration.

## Success protocol (when build + tests pass)

1. Commit:
   - `git add -A`
   - `git commit -m "<description of change>

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"`

2. Record success via gate:
   - **MCP (preferred):** Use gate MCP `pass` tool — taskId `T-200.X`, agent `ralph`, notes `<what you did>`
   - **CLI fallback:** `gate pass T-200.X --root . --agent ralph --notes '<what you did>'`

3. Write a maenifold memory note documenting what changed and why:
   - **MCP (preferred):** Use `WriteMemory` tool — title, content with `[[WikiLinks]]`, folder `mae-auto/iterations`
   - **CLI fallback:** `maenifold write --title "<title>" --content "<content>" --folder mae-auto/iterations`
   - Include `[[WikiLinks]]` to relevant concepts (e.g., `[[pipeline-handler]]`, `[[code-duplication]]`)
   - Include a `## Source` section citing the files changed

## Issue discovery

If you encounter a bug, dead code, missing test, or improvement opportunity outside your current task scope — don't fix it. Create a new gate task file for a future iteration:

1. Create `todo/T-200.{next}-{short-description}.md` with YAML frontmatter (`id`, `title`, `pipeline: lightweight`)
2. Include acceptance criteria and a description of what you found
3. Run `gate sync --root .` so gate picks it up
4. Continue with your current task

## Convergence rule

If no meaningful pending gate tasks remain (all `T-200.X` tasks are in `done/` or all remaining are blocked/non-viable), create convergence marker:

- `touch ralph/state/converged`

## Continuation rule

NEVER STOP voluntarily. Always attempt the next best improvement until converged or external iteration limit is reached.
