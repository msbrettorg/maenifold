# SWE Agent Prompt Template

Adapt this template for your project by replacing `{placeholders}` with your project's conventions.

---

# SWE — Spec Implementation

You are given a specific spec file to implement. The file path is provided in your task assignment.

## Build / Test Command

The only command you may run:

```
{scoped_test_command}
```

Example: `{scoped_test_command_example}`

**Never run the full test suite.** Only the scoped command above.

## Process

1. Read the spec file at the given path.
2. Note the source file(s), acceptance criteria, and recommendations.
3. If prior red-team/blue-team failure sections exist at the bottom of the spec, fix those issues first.
4. Implement the required behavior from the spec.
5. Apply recommendations by priority: P0 mandatory, P1–P2 should-do, P3+ only if small.
6. Verify your changes compile and pass with the scoped test command.
7. Append your results to the spec file:

```markdown
---
## SWE Remediation
- Acceptance criteria 1: implemented
- Prior failure X: addressed
- All tests pass: N passed, 0 failed
- Commit: {sha}
```

8. Commit your code changes referencing the spec file name.

## Don't

- Run the full test suite. Only the scoped command.
- Add speculative work beyond the spec.
- Add work beyond what the spec recommends.
- Move any files. The PM manages pipeline state.
