# Command Execution — Timeout Primitives and Budgets

Every command and test must self-terminate. An unbounded command hangs the iter and may discard parallel agents' work.

## Available primitives (macOS)

There is NO native `timeout` binary and NO `gtimeout` on this system.

| Primitive | Use for | Example |
|---|---|---|
| `curl --max-time N` | HTTP requests, MCP gateway calls | `curl --max-time 10 -X POST http://localhost:8080/mcp ...` |
| `perl -e 'alarm N; exec @ARGV' -- cmd args` | Arbitrary commands | `perl -e 'alarm 60; exec @ARGV' -- cargo build -p maenifold-cli` |
| Shell pattern `(cmd) & pid=$!; (sleep N && kill $pid) & wait $pid` | Backgrounded with cleanup | `(some-long-cmd) & pid=$!; (sleep 300 && kill $pid) & wait $pid` |
| Copilot CLI `bash` tool `initial_wait` | OUTER envelope for sync tool calls | `initial_wait: 120` is the floor for builds |

## Per-tool budgets

Defaults; spec author may override:

- `cargo build -p <crate>` — 600s
- `cargo test -p <crate> <test_name>` — 180s
- `curl ... /mcp ...` — `--max-time 10` for most gate calls; `--max-time 30` for discovery; `--max-time 60` for sampling
- Anything not listed — pick a number, use a primitive, never run unbounded

## No-polling-loop clause

Paste this verbatim in every worker prompt that involves polling, async-state waits, MCP discovery, daemon-reload waits, network retries, or transient-failure handling:

> Do NOT write your own polling loop or retry loop. Make ONE attempt at any operation that could fail or return empty (MCP discovery, daemon-reload, network call, etc.). If it fails or returns empty, report the failure in your WorkResult with the exact tool name + args + response, and STOP. The PM will decide whether to retry, escalate, or close — that decision is not yours.

## Enforcement

A WorkResult showing N>1 identical tool calls in succession is evidence of a polling loop. PM discards the WorkResult and re-dispatches with the clause. If the same model loops in three consecutive dispatches, escalate the model selection.
