# AcpExec

Unified ACP agent invocation for prompt execution, sequential thinking, and workflow runs.

## Parameters

- `agent` (string, required): ACP agent name from `maenifold.json`.
- `autoGrantPermissions` (bool, required): Whether ACP permission requests should be auto-granted by the native client.
- `mode` (string, optional, default: `"prompt"`): Tool execution mode. Supported values: `"prompt"`, `"think"`, `"workflow"`.
- `sessionMode` (string, optional): ACP session mode override passed to session creation. This is distinct from `mode`, which selects the AcpExec execution path.
- `prompt` (string, optional): Prompt text. Required for `mode="prompt"`. Optional for `mode="workflow"` and defaults to an empty string when omitted.
- `topic` (string, optional): Thinking topic. Required for `mode="think"`.
- `totalThoughts` (u32, optional, default: `5`): Estimated thought count for `mode="think"`.
- `workflowId` (string, optional): Workflow identifier. Required for `mode="workflow"`.
- `cwd` (string, optional): Working directory for the ACP session. Defaults to the active maenifold root when omitted.
- `model` (string, optional): Model override for the ACP session.
- `thoughtLevel` (string, optional): Thinking effort override, applied through ACP session config when the agent advertises support.
- `configOptions` (object, optional): Arbitrary ACP session config options forwarded via `session/set_config_option`.
- `allowedTools` (string[], optional): Allowed tool override for the ACP session.
- `maxTurns` (u32, optional): Maximum turns for the ACP session.
- `timeoutSeconds` (u32, optional): Session timeout in seconds.
- `context` (array, optional): Embedded context blocks sent alongside the request. Each item must include a `type` discriminator and use one of the shapes below:
  - `text`: `{ "type": "text", "text": "..." }`
  - `image`: `{ "type": "image", "data": "base64...", "mediaType": "image/png", "uri": "file:///optional.png" }`
  - `resource`: `{ "type": "resource", "uri": "memory://note", "text": "..." }`
  - `resource` with binary payload: `{ "type": "resource", "uri": "file:///diagram.bin", "blob": "base64...", "mediaType": "application/octet-stream" }`

## Behavior

All modes reuse the global ACP agent pool so repeated calls can reuse running agent processes.

### Prompt mode (`mode="prompt"`)

Sends a single prompt to the selected ACP agent and returns the response text.

Requires:

- `prompt`

### Think mode (`mode="think"`)

Runs the ACP-backed sequential thinking flow for the supplied `topic` and returns the final conclusion text.

Requires:

- `topic`

Optional:

- `totalThoughts`

### Workflow mode (`mode="workflow"`)

Runs the named workflow through the selected ACP agent and returns the workflow conclusion text.

Requires:

- `workflowId`

Optional:

- `prompt`

## Embedded Context Blocks

Use `context` to attach structured input beyond the primary prompt/topic:

- **Text blocks** add extra plain-text instructions or reference material.
- **Image blocks** attach base64-encoded images with a required media type such as `image/png`.
- **Resource blocks** embed a logical or absolute `uri` plus exactly one payload:
  - `text` for textual content
  - `blob` for base64-encoded binary content

Resource blocks are validated strictly: providing both `text` and `blob`, or neither, returns an error.

`mediaType` is the documented field name for image and resource blocks. `mimeType` is also accepted as a compatibility alias by the schema.

## Returns

- **Prompt mode**: response text from the ACP agent
- **Think mode**: final conclusion text from the thinking run
- **Workflow mode**: final conclusion text from the workflow run

If a think or workflow run completes without producing a conclusion, AcpExec returns `[no conclusion produced]`.

## Examples

Single prompt:

```json
{
  "agent": "researcher",
  "autoGrantPermissions": true,
  "prompt": "Summarize the current [[ACP]] session model."
}
```

Prompt with embedded context and session overrides:

```json
{
  "agent": "researcher",
  "autoGrantPermissions": true,
  "mode": "prompt",
  "sessionMode": "planning",
  "configOptions": {
    "verbosity": "low"
  },
  "allowedTools": ["ReadMcpResource", "GetHelp"],
  "prompt": "Compare these notes with the live [[documentation]].",
  "context": [
    {
      "type": "text",
      "text": "Focus on differences in embedded context handling."
    },
    {
      "type": "resource",
      "uri": "memory://notes/acp-summary",
      "text": "ACP sessions can advertise config options."
    }
  ]
}
```

Sequential thinking run:

```json
{
  "agent": "researcher",
  "autoGrantPermissions": true,
  "mode": "think",
  "topic": "Tradeoffs in [[ACP]] session reuse",
  "totalThoughts": 7,
  "thoughtLevel": "high"
}
```

Workflow run with image context:

```json
{
  "agent": "designer",
  "autoGrantPermissions": true,
  "mode": "workflow",
  "workflowId": "design-review",
  "prompt": "Review this screen and propose [[documentation]] updates.",
  "context": [
    {
      "type": "image",
      "data": "iVBORw0KGgoAAAANSUhEUgAA...",
      "mediaType": "image/png"
    }
  ]
}
```

## Common Errors

- Unknown `mode` values fail with: `unknown mode '...'; expected prompt, think, or workflow`
- Missing required mode-specific fields fail validation (`prompt`, `topic`, or `workflowId`)
- Invalid `context` entries report the failing array index, for example `context[1]: ...`
- Resource blocks must provide exactly one of `text` or `blob`

## Integration

- **ListAgents**: Discover valid ACP agent names and probe `modes`, `models`, and `configOptions`
- **Workflow**: Use when you need persistent maenifold-native workflow state rather than a single ACP workflow invocation
- **SequentialThinking**: Use directly when you need explicit thought-by-thought persistence instead of AcpExec's one-shot `think` mode
