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

`scripts/uat-launch.sh` is the canonical wrapper. It computes a stable task-specific port when one is not supplied, refuses an already-owned CDP port, launches the packaged binary directly with `--remote-debugging-port`, `--remote-allow-origins=*`, and a fresh `--user-data-dir`, detaches the process, verifies `/json`, and prints the exact connect command.

If the wrapper reports a port owner, list the exact owner and kill only the exact numeric PID after confirming it is the right process. Do not use name-based process cleanup. Do not hand-roll an alternate Electron launch unless the wrapper itself is being fixed.

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

## Known failure signatures

| Symptom | Likely class | Next action |
|---|---|---|
| CDP port already owned | Harness/environment | Use the wrapper's printed owner, confirm it, kill exact PID only, rerun wrapper |
| `Target.createTarget: Not supported` or WebSocket 403 | Missing `--remote-allow-origins=*` | Use `scripts/uat-launch.sh`; do not launch manually |
| CDP returns 0 pages | Packaged app/main-process startup issue | Check wrapper log path; inspect Electron main import/load failures |
| Elements missing from snapshot | Wrong target or dynamic UI | `agent-browser tab`, switch to renderer, wait for expected text, re-snapshot |
| UI needs more than 8 seconds after interaction | Product or harness failure | Capture state and investigate; do not extend the wait |
| UI compiles but button/dialog behavior fails | Real product regression | Continue interactive UAT; typecheck is not sufficient |

## Source lessons

- `AGENTS.md` CDP Testing section is the repo authority.
- `ralph/state/learnings.md` records that CDP verification catches reactive/async UI bugs blue-team misses.
- Edge commonly owns 9222; use the wrapper's stable task port or a known free port.
- Electron 111+ requires `--remote-allow-origins=*` for non-DevTools CDP clients.
