# Schedule

Manages recurring agent tasks and automations.

## Parameters

- `action` (string, required): `create`, `update`, `delete`, `list`, `get`,
  `run_now`, `logs`, or `preview`.
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
