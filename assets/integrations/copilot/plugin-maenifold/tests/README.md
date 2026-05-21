# Copilot hook red-phase tests

Plain bash test harness (chosen because `bats --version` returned `command not found` on 2026-05-21).

Run from the worktree root:

```bash
bash assets/integrations/copilot/plugin-maenifold/tests/run.sh
```

The suite stubs `curl` via `tests/bin/curl` and never calls the live maenifold daemon.
