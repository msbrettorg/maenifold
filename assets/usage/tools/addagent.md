# AddAgent

Adds or replaces a configured ACP agent entry.

## Parameters

- `name` (string, required): Agent name.
- `config` (object, required): Agent configuration.
  - `command` (string, required): Executable command.
  - `env` (object, optional): Environment variables.
  - `mcpServers` (array, optional): MCP servers exposed to the agent.

## Example

```json
{
  "name": "Researcher",
  "config": {
    "command": "agent-command"
  }
}
```
