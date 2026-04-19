# RALPH Iteration Protocol

You are running a single autonomous improvement iteration for `maenifold-rs`.

## Mission

Incrementally improve the codebase while preserving build and test stability.
Gate task files in `todo/` are the source of truth for requirements and progress.

## Read-first checklist (mandatory)

1. Read `AGENTS.md` at repo root for project conventions.
2. Check `gate status --root .` at the start of each iteration to see what is pending and ready to work on.
3. Select the next pending gate task (for example `T-200.X`) and read its task file in `todo/` for title, description, scope, and acceptance criteria.
4. Use maenifold knowledge tools to ground choices:
   - `maenifold search` for related prior work
   - `maenifold buildcontext` for surrounding concepts when useful
   - `maenifold sequentialthinking` for complex reasoning/tradeoffs

If `RALPH_TARGET_CRATE` environment variable is set and non-empty, prioritize work in that crate.

## Selection policy

- Pick the next pending gate task from `gate status --root .`.
- Treat the selected task file in `todo/` as the requirement spec.
- One improvement per iteration.

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
   - Run `gate fail T-200.X --root . --agent ralph --notes '<what went wrong>'` for the active task.
   - Append the failure pattern and lessons to `ralph/state/learnings.md`.
   - Move to next target on next iteration.

## Success protocol (when build + tests pass)

1. Commit:
   - `git add -A`
   - `git commit -m "<description of change>"`
2. Run `gate pass T-200.X --root . --agent ralph --notes '<what you did>'`.
3. Write a maenifold memory note documenting what changed and why:
   - use `maenifold write` to create a `memory://` note with rationale and outcomes

## Convergence rule

If no meaningful pending gate tasks remain, or all remaining options are blocked/non-viable, create convergence marker:

- `touch ralph/state/converged`

## Continuation rule

NEVER STOP voluntarily. Always attempt the next best improvement until converged or external iteration limit is reached.
