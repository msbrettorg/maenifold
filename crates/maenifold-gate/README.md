# maenifold-gate

`maenifold.gate` is a file-authoritative pipeline tool exposed as a native maenifold Gateway domain.

- `todo/` and `done/` task files are the source of truth.
- `.gate/gate.db` is a rebuildable cache, never the authority.
- Built-in pipelines use the explicit state-machine JSON schema.

## Quick start

```text
mkdir -p todo done
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}
```

## Gateway commands

```text
GatewayExecute domain="maenifold.gate" command="gate_Status" parameters={root: "/path/to/repo"}
GatewayExecute domain="maenifold.gate" command="gate_Append" parameters={root: "/path/to/repo", taskId: "T-1.1", agent: "swe", notes: "implemented stage work"}
GatewayExecute domain="maenifold.gate" command="gate_Pass" parameters={root: "/path/to/repo", taskId: "T-1.1", agent: "red-team", notes: "review passed"}
GatewayExecute domain="maenifold.gate" command="gate_Fail" parameters={root: "/path/to/repo", taskId: "T-1.1", agent: "blue-team", notes: "needs rework"}
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}
GatewayExecute domain="maenifold.gate" command="gate_History" parameters={root: "/path/to/repo", taskId: "T-1.1"}
GatewayExecute domain="maenifold.gate" command="gate_TaskBody" parameters={root: "/path/to/repo", taskId: "T-1.1"}
```

## Pipeline JSON schema overview

Every pipeline definition uses the same shape:

```json
{
  "schemaVersion": 1,
  "id": "sdd",
  "name": "Spec Driven Development",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "swe" } },
    "swe": { "on": { "fail": "pending", "pass": "red-team" } },
    "red-team": { "on": { "fail": "pending", "pass": "blue-team" } },
    "blue-team": { "on": { "fail": "pending", "pass": "done" } },
    "done": { "terminal": true }
  }
}
```

Rules:

- `schemaVersion` must be `1`
- `initial` must name an existing state
- terminal states cannot declare transitions
- only `pass` and `fail` outcomes exist

See `docs/gate-uml-spec.md` for the full specification.
