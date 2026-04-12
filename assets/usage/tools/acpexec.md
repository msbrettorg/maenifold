# AcpExec

Runs an ACP agent in `prompt`, `think`, or `workflow` mode.

## Parameters

- `agent` (string, required): ACP agent name from `maenifold.json`.
- `mode` (string, optional, default: `prompt`): Execution mode: `prompt`, `think`, or `workflow`.
- `sessionId` (string, optional): Load an existing ACP session instead of creating a new one.
- `cwd` (string, optional): Working directory for the ACP session.
- `autoGrantPermissions` (bool, required): Whether ACP permission requests should be auto-granted.
- `model` (string, optional): Model override for the ACP session.
- `thoughtLevel` (string, optional): Thinking effort override when supported by the agent.
- `allowedTools` (string[], optional): Allowed tools override for the session.
- `maxTurns` (integer, optional): Max turns override for the session.
- `timeoutSeconds` (integer, optional): **Execution timeout** for the prompt, think, or workflow operation. Recommended: `300-600` seconds for typical prompts and `600-1800` seconds for complex workflows or sequential thinking sessions. Default: no limit. When set, the operation is cancelled if it exceeds this duration.
- `prompt` (string, required for `prompt` and `workflow`): Prompt text to send.
- `topic` (string, required for `think`): Topic for sequential thinking.
- `totalThoughts` (integer, optional): Estimated number of thoughts for `think`.
- `workflowId` (string, required for `workflow`): Workflow identifier to run.
- `context` (array, optional): Additional ACP content blocks.

## Timeout semantics

- `timeoutSeconds` controls **execution time**, not ACP session startup or ACP connection setup.
- ACP connection timeout is configured separately in `maenifold.json` under `settings.acpConnectionTimeoutSeconds` (default: `60` seconds).
- If you omit `timeoutSeconds`, the MCP tool call can block indefinitely if the agent hangs.

## Returns

- `prompt`: Agent response text.
- `think`: Final sequential-thinking conclusion text.
- `workflow`: Workflow conclusion text.

## Example

```json
{
  "agent": "researcher",
  "mode": "prompt",
  "prompt": "Summarize the latest [[architecture]] tradeoffs.",
  "autoGrantPermissions": true,
  "timeoutSeconds": 600
}
```
