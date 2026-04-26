# MCP Resource Subscription Matrix

> Decision document for T-225.1. Defines which resource URIs support live subscriptions, polling fallback, or are non-subscribable.

## Architecture

The MCP server advertises `resources.subscribe` globally via `ServerCapabilities`. Two transport modes (stdio, HTTP) both enable subscriptions unconditionally. The subscription handler (`subscription.rs`) accepts any valid URI without checking whether the resource family actually emits updates.

## Current State

**Working emitters (2 families):**

| Family | Source | Pattern |
|--------|--------|---------|
| `asset://*` | `subscription_worker.rs` — OS file watcher | `notify` crate → 150ms debounce → per-URI emit |
| `teams://status` | `teams/runtime.rs:165-192` | State mutation → clone+compare → conditional emit |

**Dead frontend subscriptions (4):**

| URI | Component | Impact |
|-----|-----------|--------|
| `agent://sessions` | `stores/agent.ts:92` | Session picker stale |
| `workflow://sessions` | `stores/workflow.ts:136` | Workflow dashboard stale |
| `workflow://session/{id}` | `stores/workflow.ts:113` | Active workflow progress stale |
| `pipeline://sessions` | `components/PipelineSidebar.vue:91,98` | Pipeline list stale |

## Resource Matrix

### Static Resources

| URI | Current Emitter | Recommendation | Trigger Events |
|-----|-----------------|----------------|----------------|
| `asset://workflows/{id}` | YES (watcher) | **Live** (working) | File edits in assets/ |
| `asset://roles/{id}` | YES (watcher) | **Live** (working) | File edits in assets/ |
| `asset://colors/{id}` | YES (watcher) | **Live** (working) | File edits in assets/ |
| `asset://perspectives/{id}` | YES (watcher) | **Live** (working) | File edits in assets/ |
| `agent://sessions` | NO | **Live** — P1 | ACP session create/destroy |
| `teams://status` | YES (runtime) | **Live** (working) | Auth state, connection, message match |
| `teams://config` | NO | **Polling** | Explicit user action only |
| `gateway://diagnostics` | NO | **Non-subscribable** | Startup-time only |
| `graph://corpus/snapshot` | NO | **Non-subscribable** | Too large; user-triggered sync |
| `graph://corpus/communities` | NO | **Non-subscribable** | Community detection during sync |
| `graph://stats` | NO | **Polling** (30s when active) | Sync writes to SQLite |
| `pipeline://sessions` | NO | **Live** — P1 | Pipeline create/dispatch/complete |
| `schedule://jobs` | NO | **Non-subscribable** | Very low frequency |
| `workflow://sessions` | NO | **Live** — P1 | Workflow step progression |

### Resource Templates

| URI Template | Current Emitter | Recommendation | Trigger Events |
|--------------|-----------------|----------------|----------------|
| `agent://session/{id}` | NO | **Polling** | Session state changes |
| `graph://concept/{name}` | NO | **Non-subscribable** | Sync only |
| `graph://community/{id}` | NO | **Non-subscribable** | Sync only |
| `pipeline://session/{id}` | NO | **Live** — P1 | Node state transitions |
| `pipeline://node/{sid}/{nid}` | NO | **Live** — P1 | Node-level state changes |
| `schedule://job/{id}` | NO | **Non-subscribable** | Rarely changes |
| `schedule://logs/{id}` | NO | **Polling** (10s when viewing) | Logs during execution |
| `workflow://session/{id}` | NO | **Live** — P1 | Step-by-step progress |

## Decisions

### D1: Subscribe rejection model

**Decision: Option A — Server-side URI allowlist.**

`handle_subscribe()` checks the URI against a set of subscribable URI prefixes. Returns an MCP error for non-subscribable URIs. The frontend `useMcpResources.ts` already handles subscription errors gracefully.

Subscribable prefixes: `asset://`, `teams://status`, `agent://sessions`, `pipeline://`, `workflow://`.

Non-subscribable: `gateway://`, `graph://`, `schedule://`.

### D2: Frontend polling fallback

**Decision: Yes.** For non-subscribable resources the UI currently consumes, add a polling interval. For resources with no current UI subscriber, no action needed.

Polling configuration: resource-specific intervals set in the composable, not global.

### D3: Emitter implementation priority

| Priority | Family | Estimated Complexity |
|----------|--------|---------------------|
| P1 | `pipeline://sessions` + `pipeline://session/{id}` | Low — `Pipeline` tool has single entry point |
| P1 | `workflow://sessions` + `workflow://session/{id}` | Low — `Workflow` tool writes files |
| P2 | `agent://sessions` | Medium — ACP session lifecycle spans crates |

## Follow-Up Tasks

- T-225.5: Implement pipeline session emitters (pipeline://sessions, pipeline://session/{id})
- T-225.6: Implement workflow session emitters (workflow://sessions, workflow://session/{id})
- T-225.7: Add server-side subscribable URI allowlist to handle_subscribe()
- T-225.8: Frontend polling fallback for non-subscribable resources

## Key Files

| File | Purpose |
|------|---------|
| `crates/maenifold-core/src/mcp/resources/subscription.rs` | Subscribe handler — add allowlist here |
| `crates/maenifold-core/src/mcp/resources/subscription_worker.rs` | Asset watcher emitter (reference) |
| `crates/maenifold-core/src/teams/runtime.rs:165-192` | Teams emitter (reference) |
| `crates/maenifold-core/src/mcp/mod.rs:110-120` | Capability advertisement |
| `apps/desktop/src/composables/useMcpResources.ts` | UI subscription composable |
