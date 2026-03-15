# Agent Team — maenifold-rs

## Roles

| Role | Purpose | Model |
|------|---------|-------|
| **PM** | Sprint orchestration, pipeline.db state, PRD/RTM ownership, branch/release management | `claude-opus-4.6-1m` |
| **SWE** | Focused implementation — code edits, features, bug fixes, refactoring | `gpt-5.4` |
| **Red-team** | Adversarial code audit — logic flaws, spec violations, drift detection | `claude-sonnet-4.6` |
| **Blue-team** | Verification — scoped test execution, coverage gates, PASS/FAIL verdicts | `gpt-5.4` |
| **Researcher** | Deep synthesis — technical research, fact-checking, graph-grounded findings | `claude-opus-4.6` |

## Dispatch Rules

- 1 focused task per agent (quality drops with batching)
- Max 3 items per red-team audit batch
- Target 60–120s per agent; 180s hard ceiling
- A task with more than 2 file edits or 3 behavioral changes is too big — split it
- Always include `[[WikiLinks]]` in prompts for concept-as-protocol context injection
- Forbid agents from running full test suites — targeted testing only
- **Every `task` call MUST include `model` parameter explicitly. No exceptions. Never rely on defaults.**
- **Never use `claude-opus-4.6` or `claude-opus-4.6-1m` for any agent without explicit user permission.** If you believe a task requires Opus, ask the user first. Default Researcher is the sole exception, but you need permission to use it.

## Operating Rules

- **When the user asks a question, answer it immediately.** Do not launch tool calls, do not start side work, do not deflect. Stop everything and answer. Then ask if they want you to continue.
- **When the user says something, they mean exactly what they said.** Not what you think they mean. Not something adjacent. Exactly what they said.
- **Reserve full TDD pipeline (SWE→red→blue) for logic changes and code that can break.** File moves, import fixes, and config changes get a single SWE pass + build verification. Don't run ceremony on trivial work.

## Banned Models

| Model | Reason |
|-------|--------|
| `gemini-*` | Environmental instability (file locking, false test failures) |
| `claude-haiku-*` | 75% false positive rate on code audits |
| `gpt-5.1-codex-mini` | Misses subtle drift that Sonnet 4.6 catches |

## Sprint Completion Checklist

Every sprint **must** complete ALL of the following before handoff:

1. ✅ All spec files moved from `todo/` to `done/`
2. ✅ `pipeline.db` shows all tasks in terminal state
3. ✅ Targeted tests pass for all changed code
4. ✅ **Release binary rebuilt**: `cargo build --release -p maenifold-cli`
5. ✅ **Verify installed binary**: `maenifold --help` reflects all new features
6. ✅ CHANGELOG.md updated
7. ✅ RTM.md updated
8. ✅ Branch merged to main (if sprint branch used)
9. ✅ No untracked source files left uncommitted

**A sprint that ships a stale binary is a broken sprint. Period.**
