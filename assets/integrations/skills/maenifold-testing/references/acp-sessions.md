# ACP Sessions and Reconnect

Use this reference for Agents tab reconnect, ACP bridge/session lifecycle, replay, `session/load`, `session/update`, transport cleanup, or leaked `copilot --acp` processes.

## Canonical reference source

For desktop ACP session, reconnect, stream, and replay behavior, compare against the sibling checkout before changing maenifold:

```text
../acp-ui
https://github.com/formulahendry/acp-ui.git
reference commit: 27e32022a854b40b944a5872a7211d6c027cdff8
```

Important files:

- `../acp-ui/src/lib/acp-bridge.ts`
- `../acp-ui/src/stores/session.ts`
- related session list components

Do not infer ACP UI behavior from memory or SDK schemas alone. Source-check the canonical UI.

## Reconnect proof checklist

1. Build/package the desktop app.
2. Launch through `scripts/uat-launch.sh`.
3. Use agent-browser to open the Agents tab.
4. Click a saved session from a fresh snapshot ref.
5. Prove the chat view attaches, not just that the sidebar says "Active".
6. Check `agent-browser errors` and `agent-browser console`.
7. Inspect exact ACP child PIDs before/after failure cleanup when leak risk is part of the bug.

`session/load` success is not sufficient if the chat surface remains detached.

## Cleanup discipline

Failed reconnect attempts must disconnect bridges and clean up spawned ACP processes/open transports. Verify with exact PIDs. Never broad-kill by process name.

## Known failure signatures

| Symptom | Meaning | Next action |
|---|---|---|
| `Invalid params` flood during replay | Frontend validator/schema path may reject replay shape | Compare against `acp-ui` bridge handling before changing code |
| Sidebar says `Active` but main view says no active session | UI/store attachment failure | Trace saved session -> resume action -> session store -> active session view |
| `ACP connection closed` | Transport/process closed during initialize/load/replay | Capture console, app log, and ACP child PID lifecycle |
| Wrong agent name in connecting text | Selected-agent UI state may be separate from saved session agent | Trace Agents tab event payload and store resume path |

## Data-path rule

For UI-visible ACP bugs, trace source -> bridge notification -> store update -> persistence -> selector -> rendered component before scoping the fix.
