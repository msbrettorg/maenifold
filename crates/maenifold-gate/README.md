# maenifold-gate

`gate` is a file-authoritative pipeline tool.

- `todo/` and `done/` task files are the source of truth.
- `.gate/gate.db` is a rebuildable cache, never the authority.
- Built-in pipelines use the explicit state-machine JSON schema.

## Quick start

```bash
mkdir -p todo done
gate sync --root . --rebuild
```

## CLI

```bash
gate status --root .
gate append T-1.1 --root . --agent swe --notes "implemented stage work"
gate pass T-1.1 --root . --agent red-team --notes "review passed"
gate fail T-1.1 --root . --agent blue-team --notes "needs rework"
gate sync --root . --rebuild
gate history T-1.1 --root .
gate ui --root .
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
