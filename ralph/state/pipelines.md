# Pipelines UAT — Sprint 244 hot-fix follow-up

**PM:** Claude Opus 4.7 (1M context)
**Started:** 2026-04-29 06:52
**App build:** dev @ bee03582b (after T-244.74/.75/.76/.77 P0 fixes)
**Method:** CDP interactive via `agent-browser`, no faking

## User-flagged known issues (priority queue)

1. ⚠️ Window size when double-clicking item on non-draft pipeline (too big? too small?)
2. ⚠️ Can't scroll that window
3. ⚠️ Reconnect button doesn't work
4. ❓ Is it using our unified rendering engine?
5. ❓ Do we even have a unified rendering engine?
6. ❌ Graft never tested
7. ❌ Join never tested
8. ❌ ... discover more by clicking everything

## UAT log

| Time | What I clicked | What happened | Gate item filed |
|---|---|---|---|


## 06:55 — T-244.78 verified ✅

Dialog now renders at ~80vw on the inspector. Output JSON readable, Tool Provenance pretty-printed.
Screenshot: outputs/screenshots/sprint-244-uat/pipelines-04-dialog-after-fix.png

## Findings still pending

- ❓ **Unified rendering engine?** ResourceMimeDispatcher (T-244.51) ONLY handles ACP `resource` content blocks. Pipeline node Output renders as raw `<pre>`. Tool Provenance args is JSON.stringify'd manually. **NOT unified.** Should pipeline Output go through ResourcePreview or MarkdownView (with JSON code-fence)?
- ❌ Reconnect button — haven't tested
- ❌ Graft — never tested
- ❌ Join — never tested
- ❌ Fork from node — present, untested
- ❌ Rewind node — present, untested
- ❌ Promote to Memory — present, untested

## 07:01 — T-244.80 P0 filed: runner default-collapsed
Root cause: `immediate: true` watchers fire on mount, expand from stale state. No reset on pipeline-switch. Surgical fix dispatched.


## 07:03 — Graft dialog tested ✅ rendering, but "Run after" default is wrong
User clicked "Graft from node" on verify-connectivity. The dialog opens with "Run after: No upstream edge" — the originating node should auto-populate as the upstream. Otherwise Graft creates a disconnected island.
File: T-244.81 P2 (UX)

## 07:03 — Cancelling graft, looking for Reconnect button

## 2026-04-29 07:10 — Reconnect feature gap analysis (user prompt)

**User intent**: Reconnect should let user chat to refine, then OPTIONALLY take the
updated output back into the pipeline node.

**What's built today** (`SemanticZoomOverlay.vue:62-100`):
- Reconnect button shows on completed non-tool nodes with `canReconnect: true` + sessionId.
- Click → `focusOrResumeRemoteSession` → tab switches to Agents → user chats freely.
- Auto-reconnect for active (still-running) sessions.

**What's MISSING** (filed as **T-244.82 P1**):
1. No "dirty" indicator on the canvas after reconnect.
2. No way to pull the latest assistant message back into the node's stored `output`.
3. No downstream-staleness propagation when output changes.
4. No backend MCP `update_node_output` action — output mutation is read-only today.

**Architecture choice**: ship frontend-only dirty + Take/Discard for v1 (Option A).
Backend persistence + downstream cascade in a sprint-245 follow-up.


## 2026-04-29 07:18 — Reconnect scope CORRECTION

User caught me: I claimed Reconnect was "sequential-thinking only" earlier. **Wrong.**

Backend (`crates/maenifold-core/src/workflow/pipeline/view_nodes.rs:160-162`):
```rust
fn node_can_reconnect(node: &PipelineNode, state: &NodeState) -> bool {
    !matches!(node, PipelineNode::Tool { .. }) && state.acp_session_id.is_some()
}
```

**Reconnect is available on every non-tool pipeline node with an ACP session** —
workflow, prompt, sequential-thinking. Only `Tool` nodes are excluded
(they have no chat session, so reconnect would be meaningless).

T-244.82 spec updated to reflect: dirty tracking + Take/Discard applies to
ALL non-tool node types, not just sequential-thinking.

Pipeline node `analyze-cost-trends` I tried to inspect earlier IS reconnectable —
my dblclick interaction must have failed or canvas swallowed it. Worth re-testing.
