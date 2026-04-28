---
name: gate
description: "The team's gated delivery process through the native maenifold.gate Gateway domain. Use this skill when the user asks to run or set up gate, SDD, TDD, GSD, lightweight, task pipeline orchestration, `.gate/gate.db`, GatewayExecute, or custom gate process JSON."
---

# Gate

Gate is a file-authoritative pipeline tool exposed through the native `maenifold.gate` Gateway domain.

- Task files in `todo/` and `done/` are the source of truth.
- `.gate/gate.db` is a rebuildable cache only.
- The only mutating commands are `gate_Append`, `gate_Pass`, `gate_Fail`, and `gate_Sync`.
- Built-in process JSON uses the explicit state-machine schema only.

If you are changing the gate implementation itself, read [docs/gate-uml-spec.md](../../../../../docs/gate-uml-spec.md) first. That document is the implementation spec.

## How Gate Is Reached

Gate lives behind a single MCP tool surface:

```text
GatewayExecute
  domain="maenifold.gate"
  command="<gate_*>"
  parameters={ "root": "/abs/path/to/repo", … }
```

There is **no** standalone gate MCP server, no gate-specific tray icon, and no LaunchAgent. The integrated `maenifold --http` daemon hosts the `maenifold.gate` Gateway domain on the same port (default 8080) as the rest of the maenifold MCP surface.

> **Known gap (sprint 242):** the CLI (`maenifold tool …`) does **not** currently surface the gate commands. Gate is reachable only via MCP `GatewayExecute`. CLI parity is tracked as gate task **T-242.16**. Until that lands, scripts that need to drive gate from a terminal must talk to the local MCP HTTP endpoint (e.g. `curl http://127.0.0.1:8080/mcp` with the standard MCP `tools/call` envelope).

## The Seven Commands

| Conceptual operation | Domain | Command | Required parameters |
|----------------------|--------|---------|---------------------|
| List task status | `maenifold.gate` | `gate_Status` | `root` |
| Sync / rebuild cache | `maenifold.gate` | `gate_Sync` | `root` (+ optional `rebuild`) |
| Append a note (no advance) | `maenifold.gate` | `gate_Append` | `root`, `taskId`, `agent`, `notes` |
| Advance current stage with PASS | `maenifold.gate` | `gate_Pass` | `root`, `taskId`, `agent`, `notes` |
| Fail current stage | `maenifold.gate` | `gate_Fail` | `root`, `taskId`, `agent`, `notes` |
| Read task stage history | `maenifold.gate` | `gate_History` | `root`, `taskId` |
| Read task markdown body | `maenifold.gate` | `gate_TaskBody` | `root`, `taskId` |

For exact JSON schemas (including optional fields like `pipeline`, `model`, `state`, `ready`, `blocked`, `includeDone` on `gate_Status`), call:

```text
GatewayIndex parameters={"domain":"maenifold.gate","includeSchemas":true}
```

That is the source of truth — do not infer parameter names from this skill, verify against the live schema.

## Mental Model

- `todo/` is the active backlog.
- `done/` is the archive of completed task files.
- Each task file binds to one pipeline by:
  - `**Pipeline**: <name>`
  - `**Pipeline Digest**: <sha256>`
- `pending` is a dispatch state.
- The only legal outcomes are:
  - `pass`
  - `fail`

`gate_Append` records work for the active stage without advancing state.

`gate_Pass` and `gate_Fail` append canonical stage records and advance according to the active state machine.

Final-stage `gate_Pass` moves the file from `todo/` to `done/`.

Agents do not mutate gate state directly — never `mv`, `cp`, `git mv`, or hand-edit task files between `todo/` and `done/`. Always go through `gate_Pass` / `gate_Fail`.

## Built-in Processes

Gate ships four built-in pipelines:

- `sdd` — Spec Driven Development
- `tdd` — Test Driven Development
- `gsd` — Get Stuff Done
- `lightweight`

Built-in definitions live in `assets/gates/` in the source tree, and are copied into the runtime gate asset folder only if missing. **Existing runtime JSON files are never overwritten.**

Runtime gate asset directory:

- `$MAENIFOLD_ROOT/assets/gates/` (defaults to `~/maenifold/assets/gates/`)

## Process JSON Schema

Built-ins and custom pipelines use the same schema:

```json
{
  "schemaVersion": 1,
  "id": "sdd",
  "name": "Spec Driven Development",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "swe" } },
    "swe":     { "on": { "fail": "pending", "pass": "red" } },
    "red":     { "on": { "fail": "pending", "pass": "blue" } },
    "blue":    { "on": { "fail": "pending", "pass": "done" } },
    "done":    { "terminal": true }
  }
}
```

Rules:

- `schemaVersion` must be `1`.
- `initial` must point to an existing state.
- Terminal states cannot declare transitions.
- Only `pass` and `fail` outcomes exist.
- No backward compatibility with the old `stages` / `verdicts` / `transitions` format.

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
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={"root":"/path/to/repo","rebuild":true}
```

`gate_Sync`:

- creates `.gate/` if needed
- writes `.gate/gate.db`
- adds `.gate/` to `.gitignore` if missing

Do not manually commit `.gate/gate.db`.

For a fuller walkthrough, read [`references/setup-template.md`](./references/setup-template.md).

## Orchestration Loop

1. `gate_Status` — find the next ready task.
2. Dispatch the stage agent with the appropriate prompt template (see `references/`).
3. Record the result with `gate_Append` (notes only) or advance with `gate_Pass` / `gate_Fail`.
4. Repeat until `todo/` drains.

Final-stage `gate_Pass` moves the file into `done/`.

Agents do not mutate gate state directly.

## Worked Examples

Status:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Status"
  parameters={"root":"/path/to/repo"}
```

Status filtered to ready tasks only:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Status"
  parameters={"root":"/path/to/repo","ready":true}
```

Record work without advancing:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Append"
  parameters={"root":"/path/to/repo","taskId":"T-1.1","agent":"swe","notes":"implemented auth flow"}
```

Advance current stage:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Pass"
  parameters={"root":"/path/to/repo","taskId":"T-1.1","agent":"red","notes":"stage passed"}
```

Fail and follow the active state's fail edge:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Fail"
  parameters={"root":"/path/to/repo","taskId":"T-1.1","agent":"blue","notes":"needs rework"}
```

Rebuild cache:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Sync"
  parameters={"root":"/path/to/repo","rebuild":true}
```

Read history and body:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_History"
  parameters={"root":"/path/to/repo","taskId":"T-1.1"}

GatewayExecute
  domain="maenifold.gate"
  command="gate_TaskBody"
  parameters={"root":"/path/to/repo","taskId":"T-1.1"}
```

## Prompt Templates

This skill ships prompt templates for each agent role:

- [`references/swe-prompt-template.md`](./references/swe-prompt-template.md)
- [`references/red-prompt-template.md`](./references/red-prompt-template.md)
- [`references/blue-prompt-template.md`](./references/blue-prompt-template.md)
- [`references/confession-prompt-template.md`](./references/confession-prompt-template.md)

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
- `-confess` stages are run by the **same agent** who did the work (self-confession).
- `-confess` fail routes back to the work stage (not pending) — redo the work, don't restart.
- `pm-confess` fail routes to `pending` — the PM's attestation was inadequate.
- `red-team-audit` fail routes to `pending` — full restart, the pipeline's honesty was compromised.
- `red-team-audit` is a **fresh** red-team agent (not the same one from earlier) to prevent collusion.

## Recovery

Recovery is filesystem-first:

```text
GatewayExecute
  domain="maenifold.gate"
  command="gate_Sync"
  parameters={"root":"/path/to/repo","rebuild":true}
```

That reparses `todo/` and `done/`, rebuilds `.gate/gate.db`, and re-derives task state from canonical stage records.

## Health Signals

- A low but non-zero fail rate is healthy.
- Zero failures on audit stages can mean the gate is not catching anything.
- Idle slots while work remains usually means orchestration drift.
- A growing `done/` corpus means the pipeline is moving.

## When Updating This Skill

If gate internals change, keep this skill aligned with:

- [docs/gate-uml-spec.md](../../../../../docs/gate-uml-spec.md)
- `assets/gates/*.json`
- the live `GatewayIndex parameters={"domain":"maenifold.gate","includeSchemas":true}` output
- the maenifold `GatewayExecute` behavior

Do not leave stale references to obsolete commands, old cache paths, or legacy pipeline schema fields.
