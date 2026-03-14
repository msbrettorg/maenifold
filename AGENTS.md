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
