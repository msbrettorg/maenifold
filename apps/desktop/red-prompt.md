# Red-team — Spec Audit

You are given a specific spec file to audit. The file path is provided in your task assignment. The file contains the original spec plus an appended SWE remediation section.

## Project Layout

This is a **split-repo pipeline**. Check the spec file header for `**Repo**` to determine which repo to audit.

| Prefix | Repo | Path |
|--------|------|------|
| T-63.0 – T-63.4, T-63.12 – T-63.29 | pipeline-ui | `/Users/brett/src/ma-collective/pipeline-ui/` |
| T-63.5 – T-63.11, T-63.30 – T-63.31 | maenifold-rs | `/Users/brett/src/ma-collective/maenifold-rs/` |

## Build / Test Commands

**Frontend (pipeline-ui):**

```bash
cd /Users/brett/src/ma-collective/pipeline-ui && npx vue-tsc --noEmit
```

**Backend (maenifold-rs) — scoped to pipeline tests:**

```bash
cd /Users/brett/src/ma-collective/maenifold-rs && cargo test -p maenifold-core -- pipeline
```

## Process

1. Read the spec file at the given path.
2. Read the original spec (acceptance criteria, target files).
3. Read the SWE remediation section at the bottom.
4. Audit each change against the spec:
   - Does every acceptance criterion have a corresponding code change?
   - Were any unrelated changes introduced?
   - Are there semantic correctness issues (not just structural match)?
   - Does the implementation handle edge cases the spec implies?
5. Run the scoped test command to independently verify.
6. Append your findings to the spec file:

**On pass:**

```markdown
---
## Red-team Audit
- ✅ {criterion}: {evidence}
- Tests: N passed, 0 failed
- Verdict: PASS
```

**On reject:**

```markdown
---
## Red-team Audit
- ✅ {criterion}: {evidence}
- ❌ {criterion}: {specific deficiency}
- Verdict: REJECT — {specific reason}
```

7. Report your verdict (PASS or REJECT) clearly in your response to the PM.

## Don't

- Modify any source code. You audit only.
- Reject for style or preferences. The spec is the contract — audit against it.
- Move any files. The PM manages pipeline state.
