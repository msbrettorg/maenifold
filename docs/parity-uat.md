# CLI / MCP Parity UAT — Recurring Procedure

## Purpose & cadence

This is the recurring user-acceptance test for Maenifold's CLI, MCP, and
Gateway surfaces. Run it every sprint that touches any of these areas:

1. Gateway routing, Gateway schemas, Gateway domain registration, or Gateway
   command names.
2. MCP HTTP or stdio handlers, including `tools/list`, `tools/call`, resources,
   discovery, or downstream proxy behavior.
3. CLI tool dispatch, `maenifold tool <Name>`, release packaging, or tool
   registry definitions.
4. Packaged skill files under `assets/integrations/skills/`.

Run this UAT before sprint close. If it finds drift, the same sprint must either
fix the implementation or resync the skill documentation before the gate item is
allowed to close.

The canonical MCP endpoint for this procedure is:

```bash
http://127.0.0.1:8080/mcp
```

The release CLI binary used by automation is:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold
```

Use the worktree release binary for this audit. The script builds it if missing.

## Prerequisites

1. Work from the task worktree:

   ```bash
   cd /Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11
   ```

2. The MCP HTTP daemon is running on port 8080:

   ```bash
   ps aux | grep "maenifold --http"
   ```

   Expected shape:

   ```text
   brett  ... maenifold --http --port 8080
   ```

   If it is not running, start it in the environment that owns the daemon:

   ```bash
   maenifold --http --port 8080
   ```

3. Build the release CLI if it is absent:

   ```bash
   CARGO_TARGET_DIR=target cargo build --release -p maenifold-cli
   CARGO_TARGET_DIR=target cargo build --release -p maenifold-pipeline-driver
   ```

4. Confirm `jq` is available:

   ```bash
   command -v jq
   ```

   Expected output on the current UAT host:

   ```text
   /usr/bin/jq
   ```

5. Authentication is currently disabled for localhost MCP HTTP. If the daemon is
   later configured with bearer auth, the audit script automatically probes:

   ```bash
   ~/maenifold/auth/localhost.token
   ```

   and sends:

   ```text
   Authorization: Bearer <token>
   ```

## Procedure A: Surface enumeration

Procedure A establishes the complete public surface before individual tools are
called.

### A1. Top-level MCP tools

Run `tools/list` against the local HTTP MCP endpoint:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | jq -r '.result.tools | length, .[].name'
```

Expected output:

```text
7
GetHelp
ReadMcpResource
DiscoverMcpTools
GatewayIndex
GatewayExecute
GatewayBatch
GatewayIntent
```

Rules: the count is exactly seven, every entry has non-empty `name` and
`description`, and every entry has an object `inputSchema`. First-party domain
commands must not appear as individual top-level MCP tools.

### A2. Gateway domains

Run `GatewayIndex` with no arguments:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{}}}' \
  | jq -r '.result.content[0].text' \
  | jq -r '.domains[] | [.name, .command_count] | @tsv'
```

Expected native domains include:

```text
maenifold.gate      7
maenifold.graph     6
maenifold.memory    8
maenifold.system    13
maenifold.thinking  2
maenifold.workflow  2
```

Downstream proxy domains from `maenifold.json` are validated in Procedure C.

### A3. CLI tool registry

Use the release CLI from this worktree:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold tool GetHelp '{"toolName":"GatewayIndex"}'
```

Expected output starts with:

```text
# GatewayIndex
```

For every command returned by `GatewayIndex`, verify that the CLI can surface
help by command name:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold tool <Name> '{"learn":true}'
```

CLI parity is mandatory. If MCP exposes a native command the CLI cannot document
or invoke, open a P1 gate item using T-242.16 or T-242.17 as the template.

## Procedure B: Per-tool round-trip

Procedure B exercises each tool through both surfaces. Use `learn=true` first so
the audit fails on missing dispatch before it can fail on a required runtime
parameter.

### B1. CLI learn round-trip

Native command:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold \
  tool WriteMemory '{"learn":true}'
```

Expected output starts with:

```text
# WriteMemory
```

Gateway meta-tool:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold \
  tool GatewayIndex '{"learn":true}'
```

Expected output starts with either tool help or the Gateway command catalog. It
must not say `Unknown tool` and must not return a required-parameter error.

`GetHelp` minimal payload:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold \
  tool GetHelp '{"toolName":"GetHelp","learn":true}'
```

Expected output starts with:

```text
# GetHelp
```

`ReadMcpResource` read-safe URI:

```bash
/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-242.11/target/release/maenifold \
  tool ReadMcpResource '{"uri":"asset://catalog","learn":true}'
```

### B2. MCP learn or minimal-payload round-trip

For `GatewayExecute`, route a native command through the MCP surface:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"GatewayExecute","arguments":{"domain":"maenifold.system","command":"GetConfig","parameters":{"learn":true}}}}' \
  | jq -r '.result.content[0].text'
```

For a domain catalog:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{"domain":"maenifold.gate"}}}' \
  | jq -r '.result.content[0].text' \
  | jq -r '.commands[].name'
```

For `GatewayBatch`, run two read-safe commands:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"GatewayBatch","arguments":{"operation":"submit","continueOnError":true,"items":[{"domain":"maenifold.system","command":"GetConfig","parameters":{"learn":true}},{"domain":"maenifold.system","command":"MemoryStatus","parameters":{"learn":true}}]}}}' \
  | jq -r '.result.content[0].text'
```

### B3. Automated A/B execution

Run the mechanical audit:

```bash
chmod +x scripts/audit-parity.sh
./scripts/audit-parity.sh
```

Expected final line contains `PASS parity audit`.

## Procedure C: Downstream proxy + discovery

Procedure C verifies that downstream MCP servers remain discoverable.

### C1. Discover downstream tools

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"DiscoverMcpTools","arguments":{}}}' \
  | jq -r '.result.content[0].text'
```

Every configured server must report `status=ok` and a non-empty `tools` array
unless intentionally disabled in `maenifold.json`.

### C2. Cross-check discovery with GatewayIndex

For each downstream server from `DiscoverMcpTools`, run:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{"domain":"fetch"}}}' \
  | jq -r '.result.content[0].text' \
  | jq -r '.commands[].name'
```

### C3. Read-safe downstream round-trip

Pick one read-safe command per proxy. For `fetch`:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"GatewayExecute","arguments":{"domain":"fetch","command":"fetch","parameters":{"url":"https://example.com"}}}}'
```

## Procedure D: Schema drift

Procedure D compares live schemas against the packaged skill docs.

For each native domain:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{"domain":"maenifold.memory","includeSchemas":true}}}' \
  | jq -r '.result.content[0].text' \
  | jq '.commands[] | {name, schema}'
```

Repeat for `maenifold.gate`, `maenifold.graph`, `maenifold.memory`,
`maenifold.system`, `maenifold.thinking`, and `maenifold.workflow`.

Diff top-level and non-gate domain output against
`assets/integrations/skills/maenifold/SKILL.md`. Diff `maenifold.gate` against
`assets/integrations/skills/gate/SKILL.md`. Any mismatch is a documentation bug
unless the code change was unintentional.

## Procedure E: Skill resync

If Procedures A through D find drift, update both packaged skill files in the
same sprint.

Files:

```text
assets/integrations/skills/maenifold/SKILL.md
assets/integrations/skills/gate/SKILL.md
```

For the maenifold skill, update the Gateway Surface table, Domains and Commands
tables, Invocation Cheat Sheet, and stale parameter references. For the gate
skill, update The Seven Commands table, required parameters, worked examples,
and stale known-gap text.

Do not infer parameter names from previous documentation. Always use:

```bash
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{"domain":"maenifold.gate","includeSchemas":true}}}' \
  | jq -r '.result.content[0].text'
```

After updating content, refresh each skill footer:

```markdown
---

## Verification

Verified against commit `<8-char-sha>` on `<YYYY-MM-DD>` via `make audit-parity` (see `docs/parity-uat.md`).
```

## When to escalate

| Severity | Finding |
|---|---|
| P0 | `tools/list` count is not exactly 7 |
| P0 | A top-level tool disappeared or an unexpected direct domain tool appeared |
| P0 | MCP daemon cannot serve `tools/list` |
| P1 | CLI cannot document or invoke a native Gateway command |
| P1 | `GatewayIndex includeSchemas=true` differs from skill docs |
| P1 | Downstream proxy discovery disagrees with `GatewayIndex` |
| P1 | `learn=true` returns a required-parameter error for promised learn mode |
| P2 | Output examples changed but semantics are equivalent |
| P2 | Footer date or commit is stale after a resync |

Do not close a sprint touching gateway, MCP, CLI, or tool definitions until P0
and P1 findings are resolved or explicitly carried with a gate item.

## Footer

Verified against commit aa7df44a on 2026-04-28
