# Packaged Electron UAT

Use this reference for desktop UI validation, CDP, agent-browser, screenshots, red browser errors, dialog/scroll issues, or any test that must prove the real app works.

## Canonical launch path

UI verification means the packaged Electron app, not screenshots of a dev server and not typecheck alone.

```bash
cd /Users/brett/src/ma-collective/maenifold-rs
scripts/uat-launch.sh <task-id> [port]
agent-browser connect <printed-port>
agent-browser tab
agent-browser snapshot -i
```

`scripts/uat-launch.sh` is the canonical wrapper. It computes a stable task-specific port when one is not supplied, refuses an already-owned CDP port, launches the packaged binary directly with `--remote-debugging-port`, `--remote-allow-origins=*`, a fresh `--user-data-dir`, AND sets `MAENIFOLD_UAT=1` + `MAENIFOLD_UAT_FOLDER_PICKER_PATH=<tmpdir>/uat-fixture-folder`. The env vars are MANDATORY: they bypass the OS-level folder picker so agent-driven runs never hang on a native dialog that CDP cannot dismiss. The wrapper detaches the process, verifies `/json`, and prints the exact connect command.

Always grep the launch log for `[uat] MAENIFOLD_UAT mode active` before invoking any agent-browser command, to confirm the guard is active.

If the wrapper reports a port owner, list the exact owner and kill only the exact numeric PID after confirming it is the right process. Do not use name-based process cleanup. Do not hand-roll an alternate Electron launch unless the wrapper itself is being fixed.

## Backend freshness before CDP UAT (critical — iter-14 lesson)

The packaged Electron app embeds its OWN copy of the daemon binary at `apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/MacOS/maenifold`. Forge copies it from `target/release/maenifold` at `npm run package` time — it is NOT a symlink.

After any backend Rust change, the sequence is:

```bash
cargo build --release -p maenifold-cli -p maenifold-pipeline-driver
# Optional: refresh the launchd daemon at /usr/local/bin/maenifold for non-UAT clients
launchctl kickstart -k "gui/$UID/com.ma-collective.maenifold"
sleep 5
# MANDATORY: re-package the Electron app so UAT runs see the new backend
cd apps/desktop && npm run package
# THEN launch UAT
scripts/uat-launch.sh <task-id> <port>
```

Verification step before trusting any CDP UAT result against backend code:

```bash
# Confirm the packaged-app binary was refreshed
ls -la apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/MacOS/maenifold
# Mtime must be AFTER the cargo build mtime; size/inode different from the old copy.
```

Skipping the repackage leads to false-positive "backend bug" reports — the running UAT binary is stale code. Iter-14 burned 30+ minutes on T-303.4.21 v2/v3 chasing this exact illusion.

## Agent-browser discipline

Load/use the `agent-browser` Electron workflow for Electron UAT. The reliable loop is:

1. `agent-browser connect <port>`
2. `agent-browser tab`
3. select the real renderer target if needed
4. `agent-browser snapshot -i`
5. interact only with refs from the latest snapshot
6. after every click, navigation, dialog, dropdown, or dynamic re-render, wait for a specific expected element/text and re-snapshot
7. inspect `agent-browser errors` and `agent-browser console` before declaring the path clean

Refs are disposable. A ref from before a page change is not evidence and is not safe to reuse.

CDP waits are UI assertions, not sleep buffers. Use single-digit waits only: 1-5 seconds by default, 8 seconds maximum after a click, reload, navigation, or dynamic render. If the expected element/text/state does not appear inside that window, capture the snapshot, console, errors, log path, PID, and port, then classify the failure. Do not use 30s/60s waits to see whether the renderer eventually settles.

If agent-browser attaches to an empty or wrong target, inspect the wrapper's printed targets file or `curl http://127.0.0.1:<port>/json`, then switch to the real page target with `agent-browser tab`.

## Known harness gap — about:blank target

`agent-browser connect` may attach to `about:blank` instead of the maenifold renderer (T-310.2 / T-248.30). The actual renderer (`title: "maenifold"`, `url: file://...app.asar/dist/index.html`) IS exposed via CDP but agent-browser tab discovery sometimes misses it.

**Mandatory pre-flight check:** after `agent-browser connect <port>`, run `agent-browser snapshot -i`. If you see only `about:blank` chrome, fall back to:

```bash
# Find the renderer's websocket URL directly from CDP
curl -s http://127.0.0.1:<port>/json | jq '.[] | select(.title == "maenifold" or (.url // "" | contains("app.asar/dist/index.html"))) | .webSocketDebuggerUrl'
```

Do not silently skip UI verification when the renderer isn't in `agent-browser tab` output — it IS reachable. Fix tracked in T-310.2.

## Known failure signatures

| Symptom | Likely class | Next action |
|---|---|---|
| CDP port already owned | Harness/environment | Use the wrapper's printed owner, confirm it, kill exact PID only, rerun wrapper |
| `Target.createTarget: Not supported` or WebSocket 403 | Missing `--remote-allow-origins=*` | Use `scripts/uat-launch.sh`; do not launch manually |
| CDP returns 0 pages | Packaged app/main-process startup issue | Check wrapper log path; inspect Electron main import/load failures |
| Elements missing from snapshot | Wrong target or dynamic UI | `agent-browser tab`, switch to renderer, wait for expected text, re-snapshot |
| UI needs more than 8 seconds after interaction | Product or harness failure | Capture state and investigate; do not extend the wait |
| UI compiles but button/dialog behavior fails | Real product regression | Continue interactive UAT; typecheck is not sufficient |
| Backend fix shipped + daemon redeployed but UAT still hits old behavior | Stale packaged Electron binary | `cd apps/desktop && npm run package` (Forge embeds a COPY of the backend, not a symlink); then re-launch UAT |
| Folder picker hangs the run | Native dialog not bypassed | Confirm `MAENIFOLD_UAT=1` in launch env; the wrapper sets it but grep the log for `[uat] MAENIFOLD_UAT mode active` |

## Modal-handling discoveries (iter-14)

Tool-approval modals appear the FIRST time an agent uses a given MCP tool — the chat surface pauses on an `Allow once` / `Allow always` dialog. CDP recipes that send a prompt and immediately wait for the response must:

1. Snapshot after a 2-3s wait
2. Check for an `Allow once` button ref
3. If present, click it and wait another 5s before snapshotting for the actual response
4. Repeat on subsequent prompts for any newly-invoked tool

Recipes that skip this step time out with the agent stuck in a "Connecting/approval-pending" state.

## Source lessons

- This skill is the authority for CDP testing procedures. `AGENTS.md` contains the policy rules (UI verification is non-negotiable, no programmatic UAT); this skill contains the operational how-to.
- `ralph/state/learnings.md` records that CDP verification catches reactive/async UI bugs blue-team misses.
- Edge commonly owns 9222; use the wrapper's stable task port or a known free port.
- Electron 111+ requires `--remote-allow-origins=*` for non-DevTools CDP clients.

## Vue Flow drag-to-wire (T-328.6 discovery)

The pipeline designer uses Vue Flow. Handles render as `.vue-flow__handle` with `data-id` and `data-handlepos` attributes. Each node has TWO handles: `in-left-target` and `out-right-source` (or top/bottom in TB layout).

### Discover handles
```bash
agent-browser eval 'Array.from(document.querySelectorAll(".vue-flow__handle")).map(h => { const r = h.getBoundingClientRect(); return {id: h.getAttribute("data-id"), pos: h.getAttribute("data-handlepos"), x: r.x+r.width/2, y: r.y+r.height/2}; })'
```

Handle IDs follow the pattern `vue-flow-0-<nodeId>-{in-left-target|out-right-source}` (or `in-top-target` / `out-bottom-source` for TB layout).

### Wire an edge
`agent-browser drag` with CSS selectors works directly — Vue Flow accepts the synthesized pointer-event sequence:

```bash
SRC='.vue-flow__handle[data-id="vue-flow-0-<srcNodeId>-out-right-source"]'
DST='.vue-flow__handle[data-id="vue-flow-0-<dstNodeId>-in-left-target"]'
agent-browser drag "$SRC" "$DST"
agent-browser wait 1
# Verify:
agent-browser eval 'document.querySelectorAll(".vue-flow__edge").length'
# Or via snapshot — look for "Edge from <srcId> to <dstId>"
agent-browser snapshot -i | grep "Edge from"
```

The high-level `drag` command works; no need for manual `mouse move/down/up` stepping or raw CDP `Input.dispatchMouseEvent`.

### Layout-direction gotcha
The handle suffix depends on the active layout:
- LR layout (Layout LR button): `in-left-target` / `out-right-source`
- TB layout (Layout TB button): `in-top-target` / `out-bottom-source`

Query `data-handlepos` first to determine the actual direction before constructing selectors.

### What does NOT work
- Native HTML5 dragstart/drop events. Vue Flow listens for pointer events directly.
- Clicking a handle (no-op; handles only respond to drag sequences).

## Auto-dismissing toast capture (T-328.10 discovery)

Error toasts (Sonner) auto-dismiss after ~2-3 seconds. Snapshotting after the action misses them. Install a MutationObserver BEFORE the trigger:

```bash
# 1. Install observer that records every text node added to the DOM
agent-browser eval '
window.__caughtToasts = [];
const obs = new MutationObserver(muts => {
  for (const m of muts) for (const n of m.addedNodes) {
    if (n.nodeType === 1) {
      const t = n.innerText || "";
      if (t.length > 0 && t.length < 500) {
        window.__caughtToasts.push({ts: Date.now(), text: t.slice(0,400)});
      }
    }
  }
});
obs.observe(document.body, {childList: true, subtree: true});
"observer installed";
'

# 2. Trigger the failing action
agent-browser click @e31   # Create / Save / Run / whatever

# 3. Read back filtered for error-like messages
agent-browser eval 'JSON.stringify(window.__caughtToasts.filter(t => /error|fail|invalid|missing|require/i.test(t.text)).slice(-10))'
```

This captures the toast text even if it's already vanished from the visible DOM. The console/`agent-browser console` only catches `console.*` calls, not toast text — Sonner renders to a portal and toasts may not log to console.

### Console capture (complementary)
```bash
agent-browser console --clear  # reset buffer
agent-browser click @somebutton
agent-browser console | tail -30
```

Use both: console for thrown errors / explicit logs, MutationObserver for visible UI strings.
