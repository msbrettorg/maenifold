# ListAgents

Lists configured ACP agents and, optionally, probes them for live capabilities.

## Parameters

- `probe` (bool, optional, default: `false`): When `false`, return configured agents only. When `true`, spawn each agent process and attempt live capability discovery.

## Behavior

### Static listing (`probe=false`)

Reads agent definitions from `maenifold.json`, sorts them by name, and returns one object per configured agent.

This mode does **not** start agent processes. It returns the serialized config entry plus derived launch fields:

- `name`: agent name from the `agents` map key
- `command`: executable path parsed from the configured `command` string
- `args`: remaining command-line arguments parsed from the configured `command` string

Serialized config fields are preserved in the result, including fields such as:

- `env`
- `mcpServers`

If `maenifold.json` is missing or no valid agents are configured, the tool returns an empty array.

### Live discovery (`probe=true`)

Starts from the same static listing, then probes each agent by spawning it, initializing a session, and reading advertised capabilities.

Probe results are attached as a `capabilities` field on each returned agent object. Discovery is per-agent and uses an approximately 5 second timeout for each agent.

Capability discovery reports:

- `modes`
- `models`
- `configOptions`
- `agentInfo`
- `agentCapabilities`
- `style` (`modern` when config options are exposed, otherwise `legacy`)

If probing fails or times out for an individual agent, that agent still appears in the array and `capabilities` contains an error object instead of capability metadata.

## Returns

Always returns an array of agent objects.

### Static result

```json
[
  {
    "name": "researcher",
    "command": "/usr/local/bin/acp-agent",
    "args": ["serve", "--mode", "stdio"],
    "env": {
      "LOG_LEVEL": "info"
    },
    "mcpServers": [
      {
        "type": "stdio",
        "name": "filesystem",
        "command": "/usr/local/bin/fs-mcp",
        "args": []
      }
    ]
  }
]
```

### Probed result

```json
[
  {
    "name": "researcher",
    "command": "/usr/local/bin/acp-agent",
    "args": ["serve", "--mode", "stdio"],
    "env": {
      "LOG_LEVEL": "info"
    },
    "mcpServers": [],
    "capabilities": {
      "style": "modern",
      "agentInfo": {
        "name": "Research Agent",
        "version": "1.0.0",
        "title": "Research Agent"
      },
      "agentCapabilities": {
        "promptCapabilities": {
          "image": false,
          "audio": false,
          "embeddedContext": true
        },
        "sessionCapabilities": {
          "list": true,
          "loadSession": true
        }
      },
      "modes": [
        {
          "id": "default",
          "name": "Default",
          "description": "Standard prompt mode"
        }
      ],
      "models": [],
      "configOptions": []
    }
  }
]
```

### Probe timeout / failure

```json
[
  {
    "name": "slow-agent",
    "command": "/usr/local/bin/slow-agent",
    "args": [],
    "capabilities": {
      "error": "probe timed out after 5s"
    }
  }
]
```

## Examples

List configured agents only:

```json
{}
```

Probe each agent for live capability metadata:

```json
{
  "probe": true
}
```

## Use Cases

- **Inspect configured agents** before calling `AcpExec`
- **Debug agent registration** from `maenifold.json`
- **Discover supported modes/models** without hard-coding assumptions
- **Check config options** that can be set during ACP session creation

## Integration

- **AcpExec**: Use `name` from the results as the `agent` parameter
- **GetConfig**: Verify the active maenifold root if the expected `maenifold.json` is not being read
- **GetHelp**: Pair with tool docs when validating agent/tool compatibility
