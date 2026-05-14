# Schedule

Manages recurring agent tasks and automations.

## Parameters

- `action` (string, required): `create`, `update`, `delete`, `list`, `get`,
  `run_now`, `logs`, `preview`, `verify_installed`, `pause_all`,
  `resume_all`, `set_all_enabled`, or `pause_status`.
- `id` (string, required for `create`, `update`, `delete`, `get`, `run_now`,
  and `logs`): Scheduled task ID.
- `schedule` (string, required for `create` and `preview`, optional for
  `update`): Schedule expression accepted by the scheduler parser.
- `agent` (string, required for `create`, optional for `update`): ACP agent.
- `prompt` (string, required for `create`, optional for `update`): Prompt to
  run.
- `description` (string, optional): Human-readable task description.
- `enabled` (bool, optional): Whether the task is enabled.
- `autoGrantPermissions` (bool, optional): Whether ACP permissions are
  auto-granted.
- `allowedTools` (string[], optional): Tool allow-list for the scheduled run.

## Pause-all actions

`pause_all` snapshots every schedule's current `enabled` value to
`<maenifold_root>/schedules-pause-state.json`, sets enabled schedules to
disabled, and syncs every schedule with the OS scheduler so disabled schedules
with stray installed entries are uninstalled.

Return shape:

```json
{
  "action": "pause_all",
  "ok": true,
  "changedSchedules": ["daily-summary"],
  "unchangedSchedules": ["weekly-report"],
  "missingSchedules": [],
  "failed": [{ "id": "id", "error": "message", "phase": "store_update|os_sync" }],
  "message": "Schedules paused."
}
```

If a pause snapshot already exists, `pause_all` is idempotent and returns
`alreadyPaused: true` without replacing the moment-of-pause snapshot.

`resume_all` reads the moment-of-pause snapshot, writes each surviving
schedule's snapshot `enabled` value, always calls OS scheduler sync, reports
deleted schedules in `missingSchedules`, and clears the snapshot only when no
remaining schedule failed.

Return shape:

```json
{
  "action": "resume_all",
  "ok": true,
  "changedSchedules": ["daily-summary"],
  "unchangedSchedules": ["weekly-report"],
  "missingSchedules": ["deleted-while-paused"],
  "failed": [],
  "message": "Schedules resumed."
}
```

**Warning (resume_all):** `resume_all` overwrites any manual changes made to individual schedule `enabled` states while paused. The moment-of-pause snapshot is authoritative. To preserve a mid-pause edit, save the schedule explicitly after `resume_all` completes.

`set_all_enabled` requires `enabled: true|false` and bulk-sets every schedule
that differs. It rejects while pause-all is active with:
`Cannot bulk-toggle schedules while pause-all is active. Use Schedule action=resume_all to clear pause state first.`

Payload and return shape:

```json
{ "action": "set_all_enabled", "enabled": false }
```

```json
{
  "action": "set_all_enabled",
  "ok": true,
  "changedSchedules": ["daily-summary"],
  "unchangedSchedules": ["weekly-report"],
  "missingSchedules": [],
  "failed": [],
  "message": "Schedules set to enabled=false."
}
```

`pause_status` reports the current global pause state for the UI.

```json
{ "action": "pause_status", "paused": true, "pausedAt": "2026-05-14T00:00:00Z", "schedulesPaused": 2, "ok": true }
```

When not paused:

```json
{ "action": "pause_status", "paused": false, "ok": true }
```

**Warning (create / update during pause):** While pause-all is active, you cannot create or update a schedule with `enabled=true`. Resume first via `Schedule action=resume_all`, or save the schedule with `enabled=false`.

## Examples

```json
{ "action": "list" }
```

```json
{
  "action": "run_now",
  "id": "daily-summary"
}
```

```json
{
  "action": "preview",
  "schedule": "every weekday at 9am"
}
```

## Domain

MCP clients invoke this as `GatewayExecute` with domain `maenifold.system` and
command `Schedule`.
