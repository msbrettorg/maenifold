# Blue-Team Evidence Format

Blue-team approval must prove that every user-facing acceptance criterion was
checked. A build result alone is not enough to pass the stage when the task has
acceptance criteria.

## Required note structure

Use this structure in the `## Blue-Team` stage notes before recording a pass:

```markdown
Blue-team verification PASS:

### Commands run

| Command | Exit code | Evidence |
|---|---:|---|
| `cargo test -p maenifold-gate audit_blue_team_notes` | 0 | output showed 12 tests passed |
| `./scripts/audit-blue-team-notes.sh --recent 20` | 0 | no violations reported |

### Test/build result

PASS — all required commands completed successfully. If a command failed, record
the failure and do not pass the stage unless the failure is unrelated and the
reason is documented.

### Acceptance criterion checklist

| AC | Status | Evidence |
|---|---|---|
| AC1: Script scans `done/` for blue-team stages | PASS | `audit-blue-team-notes.sh`, lines 80-116; dry run on target tasks |
| AC2: Supports `--recent N` | PASS | `--recent 6` returned only newest six done files |
| AC3: Reports task IDs | PASS | sample output includes `T-05.70` and `T-214.12` |
| AC4: Flags generic notes | PASS | target-task audit flagged generic build-only notes |

### Artifacts

- Screenshot: `outputs/T-218.13/blue-team-audit.png`
- Log excerpt: command output pasted above, or stable CI link if available

### Explicit unverified items

- None
```

## Status values

Each acceptance criterion must be marked with exactly one status:

- `PASS` — directly verified with a command, source inspection, screenshot, log,
  or interactive reproduction.
- `FAIL` — checked and found not working.
- `BLOCKED` — could not be checked because a dependency, environment, tool, or
  credential was unavailable.

Do not use vague statuses such as "looks good", "probably", or "covered by
red-team".

## Evidence requirements

Evidence must be specific enough for another agent to reproduce or audit it.
Good evidence includes:

- commands and exit codes;
- source paths with line ranges or symbol names;
- test names and results;
- screenshots or logs for UI behavior;
- exact blocker details when verification cannot proceed.

Avoid generic claims without a criterion mapping, including:

- "build passes";
- "verified clean";
- "red-team confirmed";
- "tests pass";
- "all checks pass".

Those phrases may appear only when accompanied by the full evidence matrix above.

## Blocked or partial verification

If any acceptance criterion is `FAIL` or `BLOCKED`, the blue-team stage must not
pass. Record the evidence matrix, list the failing or blocked criteria under
`Explicit unverified items`, and return the stage according to the pipeline rules.

Example:

```markdown
### Acceptance criterion checklist

| AC | Status | Evidence |
|---|---|---|
| AC1: Button renders in Electron | PASS | screenshot `outputs/T-12/button.png` |
| AC2: Keyboard shortcut works | BLOCKED | packaged app did not launch; see error log |

### Explicit unverified items

- AC2 is blocked by packaged-app launch failure. Blue-team cannot pass until the
  app launches and the shortcut is tested.
```

Partial verification is useful evidence, but it is not a pass.
