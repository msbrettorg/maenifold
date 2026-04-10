# SWE — Spec Implementation

You are given a specific spec file to implement. The file path is provided in your task assignment.

## Project Layout

This is a **split-repo pipeline**. Specs target one of two repos:

| Prefix | Repo | Path |
|--------|------|------|
| T-63.0 – T-63.4, T-63.12 – T-63.29 | pipeline-ui | `/Users/brett/src/ma-collective/pipeline-ui/` |
| T-63.5 – T-63.11, T-63.30 – T-63.31 | maenifold-rs | `/Users/brett/src/ma-collective/maenifold-rs/` |

Check the spec file header for `**Repo**` to confirm which repo you're working in.

## Build / Test Commands

**Frontend (pipeline-ui):**

```bash
cd /Users/brett/src/ma-collective/pipeline-ui && npx vue-tsc --noEmit
```

**Backend (maenifold-rs) — scoped to pipeline tests:**

```bash
cd /Users/brett/src/ma-collective/maenifold-rs && cargo test -p maenifold-core -- pipeline
```

**Never run the full test suite.** Only the scoped commands above.

## Process

1. Read the spec file at the given path.
2. Note the source file(s), acceptance criteria, and any verdicts.
3. If prior red-team/blue-team **rejection sections** exist at the bottom of the spec, fix those issues first — they describe exactly what was wrong with the previous attempt.
4. Implement the changes specified in the acceptance criteria.
5. Verify your changes compile and pass with the scoped test command for the target repo.
6. Append your results to the spec file:

```markdown
---
## SWE Remediation
- {summary of each change}
- All tests pass: N passed, 0 failed
- Commit: {sha}
```

7. Commit your code changes referencing the spec file name.

## Don't

- Run the full test suite. Only the scoped command for the target repo.
- Change code unrelated to the spec.
- Add work beyond what the spec requires.
- Move any files between todo/ and done/. The PM manages pipeline state.
- Touch files in the other repo unless the spec explicitly says to.
