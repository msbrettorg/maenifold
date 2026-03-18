<!-- Minimal local copy of the official ACP docs. Source: https://agentclientprotocol.com/protocol/tool-calls -->

# Tool Calls

> How Agents report tool call execution

Tool calls represent actions that language models request Agents to perform during a prompt turn. When an LLM determines it needs to interact with external systems—like reading files, running code, or fetching data—it generates tool calls that the Agent executes on its behalf.

Agents report tool calls through `session/update` notifications, allowing Clients to display real-time progress and results to users.

While Agents handle the actual execution, they may leverage Client capabilities like permission requests or file system access to provide a richer, more integrated experience.

## Creating

When the language model requests a tool invocation, the Agent **SHOULD** report it to the Client:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "sess_abc123def456",
    "update": {
      "sessionUpdate": "tool_call",
      "toolCallId": "call_001",
      "title": "Reading configuration file",
      "kind": "read",
      "status": "pending"
    }
  }
}
```

The `kind` categories include:

- `read`
- `edit`
- `delete`
- `move`
- `search`
- `execute`
- `think`
- `fetch`
- `other`

## Updating

As tools execute, Agents send updates to report progress and results.

Updates use the `session/update` notification with `tool_call_update`.

All fields except `toolCallId` are optional in updates. Only the fields being changed need to be included.

## Requesting Permission

The Agent **MAY** request permission from the user before executing a tool call by calling the `session/request_permission` method.

The Client responds with the user's decision.

Clients **MAY** automatically allow or reject permission requests according to the user settings.

If the current prompt turn gets cancelled, the Client **MUST** respond with the `cancelled` outcome.

### Permission Options

Each permission option provided to the Client contains:

- `optionId`
- `name`
- `kind`

Kinds:

- `allow_once`
- `allow_always`
- `reject_once`
- `reject_always`

## Status

Tool calls progress through different statuses during their lifecycle:

- `pending`
- `in_progress`
- `completed`
- `failed`

## Content

Tool calls can produce different types of content:

### Regular Content

Standard content blocks like text, images, or resources.

### Diffs

File modifications shown as diffs.

### Terminals

Live terminal output from command execution.

When a terminal is embedded in a tool call, the Client displays live output as it's generated and continues to display it even after the terminal is released.

## Following the Agent

Tool calls can report file locations they're working with, enabling Clients to implement “follow-along” features that track which files the Agent is accessing or modifying in real-time.
