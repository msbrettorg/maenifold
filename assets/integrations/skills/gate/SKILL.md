---
name: gate
description: "The team's gated delivery process through the native maenifold.gate Gateway domain. Use this skill when the user asks to run or set up gate, SDD, TDD, GSD, lightweight, task pipeline orchestration, `.gate/gate.db`, GatewayExecute, or custom gate process JSON."
---

# Gate

Gate is a file-authoritative pipeline tool exposed through the native `maenifold.gate` Gateway domain.

- Task files in `todo/` and `done/` are the source of truth.
- `.gate/gate.db` is a rebuildable cache only.
- The only mutating commands are `append`, `pass`, `fail`, and `sync`.
- Built-in process JSON uses the new explicit state-machine schema only.

If you are changing the gate implementation itself, read [docs/gate-uml-spec.md](../../../../../docs/gate-uml-spec.md) first. That document is the implementation spec.

## Current Surface

Gate is accessed with `GatewayExecute` against the integrated domain `maenifold.gate`.
The existing `maenifold --http` daemon hosts this Gateway surface, normally via the maenifold MCP HTTP endpoint on port `8080`.
Gate is not run as its own MCP server and has no gate-specific tray, tooltip, LaunchAgent, or open-at-login service mode.

Gateway commands:

- `gate_Status` — list task status
- `gate_Sync` — validate and rebuild cache
- `gate_Append` — record notes without advancing
- `gate_Pass` — advance with pass
- `gate_Fail` — advance with fail
- `gate_History` — read task stage history
- `gate_TaskBody` — read task markdown

Access pattern:

```text
GatewayExecute
  domain: "maenifold.gate"
  command: "gate_Status"
  parameters: {root: "/path/to/repo"}
```

## Mental Model

- `todo/` is the active backlog.
- `done/` is the archive of completed task files.
- Each task file binds to one pipeline by:
  - `**Pipeline**: <name>`
  - `**Pipeline Digest**: <sha256>`
- `pending` is a dispatch state.
- Legal outcomes are only:
  - `pass`
  - `fail`

`append` records work for the active stage without advancing state.

`pass` and `fail` append canonical stage records and advance according to the active state machine.

## Built-in Processes

Gate currently ships four built-ins:

- `sdd`
- `tdd`
- `gsd`
- `lightweight`

Built-in definitions live in `assets/gates/`, then are copied into the runtime gate asset folder only if missing. Existing runtime JSON files are never overwritten.

Runtime gate asset directory:

- `$MAENIFOLD_ROOT/assets/gates/` (defaults to `~/maenifold/assets/gates/`)

## Process JSON Schema

Built-ins and custom pipelines use the same schema shape:

```json
{
  "schemaVersion": 1,
  "id": "sdd",
  "name": "Spec Driven Development",
  "initial": "pending",
  "states": {
    "pending": {
      "on": {
        "pass": "swe"
      }
    },
    "swe": {
      "on": {
        "fail": "pending",
        "pass": "red"
      }
    },
    "red": {
      "on": {
        "fail": "pending",
        "pass": "blue"
      }
    },
    "blue": {
      "on": {
        "fail": "pending",
        "pass": "done"
      }
    },
    "done": {
      "terminal": true
    }
  }
}
```

Rules:

- `schemaVersion` must be `1`
- `initial` must point to an existing state
- terminal states cannot declare transitions
- only `pass` and `fail` outcomes exist
- no backward compatibility with the old `stages` / `verdicts` / `transitions` format

## Setup

Minimum project layout:

```text
project/
├── todo/
├── done/
└── .gate/        # created by gate_Sync when needed
```

Initialize a new pipeline root:

```text
mkdir -p todo done
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}
```

`gate_Sync` now:

- creates `.gate/` if needed
- writes `.gate/gate.db`
- adds `.gate/` to `.gitignore` if missing

Do not manually commit `.gate/gate.db`.

For a fuller walkthrough, read `references/setup-template.md`.

## Orchestration Loop

Loop:

1. Run `gate_Status` through `GatewayExecute`
2. Pick the next ready task
3. Dispatch the stage agent
4. Record the result with:
   - `gate_Append`
   - `gate_Pass`
   - `gate_Fail`
5. Repeat until `todo/` drains

Final-stage `gate_Pass` moves the file into `done/`.

Agents do not mutate gate state directly.

## Gateway Commands

```text
# status
GatewayExecute domain="maenifold.gate" command="gate_Status" parameters={root: "/path/to/repo"}

# record work without advancing
GatewayExecute domain="maenifold.gate" command="gate_Append" parameters={root: "/path/to/repo", taskId: "T-1.1", agent: "swe", notes: "implemented auth flow"}

# advance current stage
GatewayExecute domain="maenifold.gate" command="gate_Pass" parameters={root: "/path/to/repo", taskId: "T-1.1", agent: "red", notes: "stage passed"}

# return according to the active state's fail edge
GatewayExecute domain="maenifold.gate" command="gate_Fail" parameters={root: "/path/to/repo", taskId: "T-1.1", agent: "blue", notes: "needs rework"}

# rebuild cache
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}

# read history and task body
GatewayExecute domain="maenifold.gate" command="gate_History" parameters={root: "/path/to/repo", taskId: "T-1.1"}
GatewayExecute domain="maenifold.gate" command="gate_TaskBody" parameters={root: "/path/to/repo", taskId: "T-1.1"}
```

## Gateway Usage

`maenifold.gate` exposes the gate operations as native Gateway commands:

- `gate_Status`
- `gate_Append`
- `gate_Pass`
- `gate_Fail`
- `gate_Sync`
- `gate_History`
- `gate_TaskBody`

Use `GatewayExecute` through the existing maenifold MCP connection:

```text
GatewayExecute domain="maenifold.gate" command="gate_Status" parameters={root: "/path/to/repo"}
```

The maenifold HTTP MCP host is provided by `maenifold --http`; the gate domain does not require or provide its own service process.

## Prompt Templates

This skill ships prompt templates for each agent role:

- `references/swe-prompt-template.md`
- `references/red-prompt-template.md`
- `references/blue-prompt-template.md`
- `references/confession-prompt-template.md`

Use them as starting points, then adapt them to your repo and test commands.

### Stage → Prompt Mapping (sdd-c / tdd-c)

For confession-enhanced processes (`sdd-c`, `tdd-c`), use this mapping to dispatch the correct prompt for each stage:

| Gate Stage | Prompt Template | Agent | Role |
|------------|----------------|-------|------|
| `pending` | (PM dispatches) | PM | Dispatch |
| `swe` | `swe-prompt-template.md` | SWE | Implementation |
| `swe-confess` | `confession-prompt-template.md` (SWE section) | Same SWE | Self-confession |
| `red-team` | `red-prompt-template.md` | Red-team | Adversarial audit |
| `red-team-confess` | `confession-prompt-template.md` (Red-team section) | Same red-team | Self-confession |
| `blue-team` | `blue-prompt-template.md` | Blue-team | Verification |
| `blue-team-confess` | `confession-prompt-template.md` (Blue-team section) | Same blue-team | Self-confession |
| `pm-confess` | `confession-prompt-template.md` (PM section) | PM | PM self-confession |
| `red-team-audit` | `confession-prompt-template.md` (Audit section) | Fresh red-team | Falsify PM confession |

**Rules:**
- `-confess` stages are run by the **same agent** who did the work (self-confession)
- `-confess` fail routes back to the work stage (not pending) — redo the work, don't restart
- `pm-confess` fail routes to `pending` — the PM's attestation was inadequate
- `red-team-audit` fail routes to `pending` — full restart, the pipeline's honesty was compromised
- `red-team-audit` is a **fresh** red-team agent (not the same one from earlier) to prevent collusion

## Recovery

Recovery is filesystem-first:

```text
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}
```

That reparses `todo/` and `done/`, rebuilds `.gate/gate.db`, and re-derives task state from canonical stage records.

## Health Signals

- Low but non-zero fail rate is healthy.
- Zero failures on audit stages can mean the gate is not catching anything.
- Idle slots while work remains usually means orchestration drift.
- A growing `done/` corpus means the pipeline is moving.

## When Updating This Skill

If gate internals change, keep this skill aligned with:

- [docs/gate-uml-spec.md](../../../../../docs/gate-uml-spec.md)
- `assets/gates/*.json`
- the native `maenifold.gate` Gateway command surface
- the maenifold `GatewayExecute` behavior

Do not leave stale references to obsolete commands, old cache paths, or legacy pipeline schema fields.
