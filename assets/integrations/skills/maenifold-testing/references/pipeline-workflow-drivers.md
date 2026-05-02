# Pipeline, Workflow, and Driver Runtime

Use this reference for workflow/pipeline behavior, DAG execution, pipeline UI, driver leaks, stale runtime binaries, and task/pipeline debugging.

## Two-binary deployment surface

Workflow/pipeline changes often affect two binaries:

- `maenifold-cli`
- `maenifold-pipeline-driver`

If touched code can run in either process, rebuild both before runtime validation.

```bash
cargo build --release -p maenifold-cli
cargo build --release -p maenifold-pipeline-driver
```

Verify the running binary, not just the source tree. A stale packaged app or stale driver can reproduce old behavior after source is fixed.

## Stale driver cleanup

Driver processes can persist for the lifetime of in-flight pipeline executions. Inspect exact PIDs and terminate only the specific stale process IDs. Never broad-kill by process name.

## Pipeline UI data-path rule

For pipeline UI bugs, trace the full path:

```text
producer -> persistence -> read API -> frontend selector -> rendered control
```

Do not stop at "backend persisted it" if the frontend filters or selectors exclude that data shape.

## Runtime freshness

Before declaring a task stale based on runtime behavior:

1. Rebuild from current source.
2. Re-test against that built artifact.
3. Compare running binary/path/hash if packaged output differs.

Do not trust a packaged binary as ground truth when source and runtime may be out of sync.
