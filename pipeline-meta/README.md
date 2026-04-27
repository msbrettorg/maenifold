# pipeline-meta

`pipeline-meta` is the DAG-as-PM delivery harness. An agent chooses what to run; the pipeline owns execution flow, stage routing, remediation edges, and gate tool nodes.

## Safe canary

From the repository root:

```text
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo"}
./pipeline-meta/pipeline-meta.sh --dry-run --task T-218.42
```

The dry-run must log root-level paths:

- `pipeline-meta/logs/<timestamp>.log`
- `pipeline-meta/otel/<timestamp>.jsonl`
- `pipeline-meta/templates/<pipeline>-v1.json`

If template rendering fails, dry-run exits non-zero instead of pretending it would execute.

## Selection controls

```bash
./pipeline-meta/pipeline-meta.sh --dry-run --task T-218.42
./pipeline-meta/pipeline-meta.sh --dry-run --pipeline lightweight --limit 3
./pipeline-meta/pipeline-meta.sh --task T-218.42
```

Use `--task` for the first live run. Do not run the whole backlog until a single-task canary has completed end-to-end.

## Templates

Templates live in `pipeline-meta/templates/`:

- `lightweight-v1.json`
- `sdd-v1.json`
- `slc-v1.json`
- `tdd-v1.json`

Prompt nodes specify explicit models. Gate tool nodes include `root: "{{inputs.worktreePath}}"` and record transitions as `pipeline-meta`, because the DAG runner is the PM-shaped execution controller. Templates include an initial gate dispatch node so prompt stages run while the task is in the matching gate state.

## Outputs

- `logs/` — timestamped shell logs
- `otel/` — Copilot OTEL JSONL path
- `metrics/` — per-task and summary metrics
- `meta/` — meta-optimization snapshots, verdicts, and archived metrics
- `verdicts/` — reserved for future human-readable verdict artifacts
