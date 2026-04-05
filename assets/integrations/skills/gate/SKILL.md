---
name: gate
description: "The team's gated delivery process and gate CLI/MCP runtime. Use this skill when the user asks to run or set up gate, SDD, TDD, GSD, lightweight, task pipeline orchestration, `.gate/gate.db`, `gate --mcp`, `gate --http`, tray/runtime behavior, open-at-login, or custom gate process JSON."
---

# Gate

`gate` is a file-authoritative pipeline tool.

- Task files in `todo/` and `done/` are the source of truth.
- `.gate/gate.db` is a rebuildable cache only.
- The only mutating commands are `append`, `pass`, `fail`, and `sync`.
- Built-in process JSON uses the new explicit state-machine schema only.

If you are changing the gate implementation itself, read [docs/gate-uml-spec.md](../../../../../docs/gate-uml-spec.md) first. That document is the implementation spec.

## Current Surface

CLI commands:

- `gate status`
- `gate append`
- `gate pass`
- `gate fail`
- `gate sync`

Service modes:

- `gate --mcp`
  MCP over stdio
- `gate --http --port <n>`
  MCP over streamable HTTP at `/mcp`

On macOS, both MCP service modes create a tray icon per running instance. HTTP mode also supports open-at-login via a LaunchAgent plist.

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

Built-in definitions are bundled into `maenifold-gate`, then copied into the runtime gate asset folder only if missing. Existing runtime JSON files are never overwritten.

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
└── .gate/        # created by gate sync when needed
```

Initialize a new pipeline root:

```bash
mkdir -p todo done
gate sync --root . --rebuild
```

`gate sync` now:

- creates `.gate/` if needed
- writes `.gate/gate.db`
- adds `.gate/` to `.gitignore` if missing

Do not manually commit `.gate/gate.db`.

For a fuller walkthrough, read `references/setup-template.md`.

## Orchestration Loop

Loop:

1. Run `gate status --root . --json`
2. Pick the next ready task
3. Dispatch the stage agent
4. Record the result with:
   - `gate append`
   - `gate pass`
   - `gate fail`
5. Repeat until `todo/` drains

Final-stage `gate pass` moves the file into `done/`.

Agents do not mutate gate state directly.

## Common Commands

```bash
# status
gate status --root .
gate status --root . --json

# record work without advancing
gate append T-1.1 --root . --agent swe --notes "implemented auth flow"

# advance current stage
gate pass T-1.1 --root . --agent red --notes "stage passed"

# return according to the active state's fail edge
gate fail T-1.1 --root . --agent blue --notes "needs rework"

# rebuild cache
gate sync --root . --rebuild
```

## MCP Usage

`gate` exposes the same five operations over MCP:

- `status`
- `append`
- `pass`
- `fail`
- `sync`

Use:

```bash
gate --mcp
gate --http --port 8081
```

HTTP MCP serves at `http://127.0.0.1:<port>/mcp`.

Tray behavior on macOS:

- stdio tooltip: `gate MCP stdio`
- HTTP tooltip: `gate MCP :<port>`
- open-at-login is available only for HTTP mode

## Prompt Templates

This skill ships SDD-oriented prompt templates:

- `references/swe-prompt-template.md`
- `references/red-prompt-template.md`
- `references/blue-prompt-template.md`

Use them as starting points, then adapt them to your repo and test commands.

## Recovery

Recovery is filesystem-first:

```bash
gate sync --root . --rebuild
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
- `crates/maenifold-gate/assets/*.json`
- the actual CLI help for `gate`
- the actual service-mode behavior for `--mcp` and `--http`

Do not leave stale references to obsolete commands, old cache paths, or legacy pipeline schema fields.
