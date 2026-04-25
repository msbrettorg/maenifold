# Gate cancel taxonomy

Gate cancellation is an administrative close action, not evidence that a task
passed its delivery pipeline. Cancelled tasks should record why they left active
work and, when risk remains, which follow-up task or external reference owns
that risk.

## Cancellation categories

Use exactly one leading category in the `## Cancelled` notes:

- `superseded-by: T-XXX.Y` — the work was replaced by a newer task. Use this
  when the original task should not continue because the successor owns the
  same or broader outcome.
- `false-positive` — the reported problem does not exist, was already fixed, or
  was based on an incorrect read of the system.
- `blocked-upstream: <reference>` — an external dependency prevents completion.
  The reference may be an upstream issue, vendor release, protocol change, or
  other durable tracking pointer.
- `duplicate: T-XXX.Y` — this task is identical to another task that remains the
  canonical owner.
- `out-of-scope` — the work is outside current project scope and no follow-up is
  planned.
- `epic-complete` — the task is an epic/container and all child work is resolved
  or explicitly tracked elsewhere.

If none of these categories fits, prefer leaving the task active until a clearer
disposition exists. Do not use cancellation to hide unresolved project-owned
risk.

## Format in task markdown

The category should be the first sentence of the `- Notes:` value in the
`## Cancelled` section. Additional rationale and follow-up links can follow.

### Superseded work

```markdown
## Cancelled

- Stage ID: cancelled
- Record Kind: cancel
- Agent: pm
- Recorded At: 2026-04-25T00:00:00Z
- Outcome: CANCEL
- Notes: superseded-by: T-218.19. The replacement task broadens the audit to
  include done metrics and unresolved-risk links.
```

### False positive

```markdown
## Cancelled

- Stage ID: cancelled
- Record Kind: cancel
- Agent: pm
- Recorded At: 2026-04-25T00:00:00Z
- Outcome: CANCEL
- Notes: false-positive. The button is already wired through ACP content-block
  capability checks; no product defect remains.
```

### Upstream blocker

```markdown
## Cancelled

- Stage ID: cancelled
- Record Kind: cancel
- Agent: pm
- Recorded At: 2026-04-25T00:00:00Z
- Outcome: CANCEL
- Notes: blocked-upstream: rmcp session rejection policy. Local fixes either do
  not intercept the failing path or break stateless clients; refile when rmcp
  exposes a configurable policy or the exact HTTP method/path is traced.
```

### Duplicate

```markdown
## Cancelled

- Stage ID: cancelled
- Record Kind: cancel
- Agent: pm
- Recorded At: 2026-04-25T00:00:00Z
- Outcome: CANCEL
- Notes: duplicate: T-218.20. T-218.20 is the canonical task and already
  contains the same acceptance criteria.
```

### Out of scope

```markdown
## Cancelled

- Stage ID: cancelled
- Record Kind: cancel
- Agent: pm
- Recorded At: 2026-04-25T00:00:00Z
- Outcome: CANCEL
- Notes: out-of-scope. The requested process change is outside this project and
  no follow-up is planned.
```

### Completed epic/container

```markdown
## Cancelled

- Stage ID: cancelled
- Record Kind: cancel
- Agent: pm
- Recorded At: 2026-04-25T00:00:00Z
- Outcome: CANCEL
- Notes: epic-complete. All child tasks shipped or were explicitly disposed; the
  container task is closed without implying it passed implementation stages.
```

## Audit of recent cancelled tasks

Audit command used:

```bash
grep -l "Cancelled" done/T-*.md
```

The command finds both real cancellation sections and task specs that mention
`## Cancelled` as example text. The table below distinguishes those cases.

| Task | Current location | Observed cancellation note | Classification | Follow-up / disposition |
| --- | --- | --- | --- | --- |
| T-05.68 | `done/T-05.68.md` | Superseded by T-05.70 replacing the GateTaskTable. | `superseded-by: T-05.70` | No separate follow-up needed; successor owns the work. |
| T-05.86 | `done/T-05.86.md` | Blue-team filed from grep counts without reading skip rationales. | `false-positive` | No follow-up needed. |
| T-179.34 | `done/T-179.34.md` | Sprint 201 onboarding wizard work was superseded by T-202.2, T-202.3, and T-202.4. | `superseded-by: T-202.2` plus related successors T-202.3/T-202.4 | No follow-up needed beyond the successor remediation tasks. |
| T-202.11 | `done/T-202.11.md` | MindDashboard placeholder-data issue disappeared after reverting to HomeDashboard. | `false-positive` | No follow-up needed; affected UI is dead code. |
| T-202.17 | `done/T-202.17.md` | Attachment buttons were verified as wired and capability-gated. | `false-positive` | No follow-up needed. |
| T-202.5 | `done/T-202.5.md` | Process improvement work declared outside scope by user direction. | `out-of-scope` | No follow-up planned in that task. |
| T-202.7 | `done/T-202.7.md` | Cancel command task superseded by T-202.2 through T-202.4. | `superseded-by: T-202.2` plus related successors T-202.3/T-202.4 | No follow-up needed beyond successor tasks. |
| T-203.0 | `done/T-203.0.md` | MCP Apps host epic closed after children shipped; T-203.7 separately cancelled. | `epic-complete` | Epic closure is acceptable because child risk is separately visible. |
| T-203.7 | `done/T-203.7.md` | Two local fixes failed; root cause appears to be upstream rmcp behavior around stale MCP session headers. | `blocked-upstream: rmcp session rejection policy` | See [T-203.7 unresolved-risk disposition](#t-2037-unresolved-risk-disposition). |
| T-205.705 | `done/T-205.705-missing-view-all-agents-button.md` | Agents tile already navigates to Agents tab; separate button is redundant. | `false-positive` | UAT TC-039 marked N/A; no task follow-up needed. |
| T-207.0 | `done/T-207.0.md` | Epic container closed while remaining subtasks/backlog items are tracked individually. | `epic-complete` | Acceptable only because remaining work is named in the note: T-207.8, T-207.9, T-207.7. |
| T-210.0 | `done/T-210.0.md` | Teams Monitor epic closed after all 11 subtasks shipped. | `epic-complete` | No follow-up needed. |
| T-211.0 | `done/T-211.0.md` | Scheduled Tasks epic closed after all 11 subtasks shipped. | `epic-complete` | No follow-up needed. |
| T-212.0 | `done/T-212.0.md` | ralph-console epic closed after all 3 subtasks shipped. | `epic-complete` | No follow-up needed. |
| T-174.6 | `done/T-174.6.md` | No actual `## Cancelled` section; grep matched the word `Cancelled` in retry-classifier requirements. | Not cancelled | No cancellation classification. |
| T-202.8 | `done/T-202.8.md` | No actual cancellation; task spec intentionally contains a fenced-code `## Cancelled` parser regression case. | Not cancelled | No cancellation classification. |

### Requested T-209 audit notes

The requested T-209.0 and T-209.7 cancelled files were not present in `done/`
in this worktree. They remain in `todo/`:

- `todo/T-209.0-epic-pipeline-driven-delivery.md`
- `todo/T-209.7-validate-on-real-tasks.md`

Classification based on the requested audit descriptions and the task contents:

- T-209.7: classify as `out-of-scope` if cancelled because the original T-02.x
  validation targets are exhausted and no replacement validation scope is
  selected. If a replacement validation task is created against current tasks,
  classify as `superseded-by: <new task id>` instead.
- T-209.0: classify as `epic-complete` if cancelled as a completed
  pipeline-driven delivery epic after all child tasks are shipped or explicitly
  disposed. In this worktree T-209.7 is still active, so T-209.0 should not yet
  be treated as an audited cancelled epic here even though the requested audit
  description says "recently cancelled — epic complete."

## T-203.7 unresolved-risk disposition

T-203.7 has unresolved risk. The product still has a known intermittent failure
mode: after idle periods, MCP HTTP calls may fail with `Session not found` when
the client sends a stale MCP session header or opens the stale stream path. The
task history records two rejected local fixes:

1. Switching from `NeverSessionManager` to `LocalSessionManager` made the server
   stateful and broke existing stateless clients.
2. Adding an `IgnoreSessionManager` did not intercept the failing rmcp code path
   in stateless mode.

The accepted operational workaround is to use the CLI fallback
`gate sync --root . --rebuild` when MCP sync/status fails after idle. That
workaround does not remove the underlying MCP reliability risk.

Recommended disposition:

- Classify T-203.7 as `blocked-upstream: rmcp session rejection policy`.
- Refile a new task only when one of these is true:
  - rmcp exposes a configurable session rejection or stale-session policy;
  - the exact HTTP method/path causing the stale-session 404 is reproduced and
    can be handled locally without breaking stateless clients;
  - the project decides the CLI fallback is insufficient for production
    reliability.

Until one of those triggers occurs, the risk is accepted with a documented CLI
fallback rather than hidden as completed work.
