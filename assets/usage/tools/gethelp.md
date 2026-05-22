# GetHelp

Loads complete command or tool documentation from the runtime
`~/maenifold/assets/usage/tools/{toolname}.md` tree.

`GetHelp` itself is a direct public MCP tool. First-party operations such as
`ReadMemory`, `SearchMemories`, `Pipeline`, and `GetConfig` are documented by
their command names but are invoked over MCP through `GatewayExecute`.

## Parameters

- `toolName` (string, required): Public tool or gateway command name. Example:
  `"GatewayIndex"`, `"ReadMemory"`, `"SearchMemories"`, `"BuildContext"`

## Returns

```json
{
  "content": "# WriteMemory\n\nCreates knowledge files...",
  "toolName": "WriteMemory",
  "helpFilePath": "~/maenifold/assets/usage/tools/writememory.md"
}
```

## Example

```json
{
  "toolName": "SequentialThinking"
}
```

## Available Help Topics

For the **live, authoritative** list of every public MCP tool, gateway domain,
and command available on this daemon — including upstream MCP servers attached
through the gateway — call `GatewayIndex` (with no arguments for the domain
list, or with `domain` + `includeSchemas: true` to inspect a single domain's
commands and parameter schemas).

The shape served by `GetHelp` is the same regardless of where a command lives:
pass any public MCP tool name (`GatewayIndex`, `GatewayExecute`, `GatewayBatch`,
`GatewayIntent`, `GetHelp`, `ReadMcpResource`, `DiscoverMcpTools`,
`DiscoverMcpPrompts`, `GetMcpPrompt`) **or** any gateway command name surfaced
by `GatewayIndex` (`SearchMemories`, `BuildContext`, `Workflow`, `Pipeline`,
`gate_Status`, …) and `GetHelp` will return the corresponding markdown if a
help file exists.

Help-file presence is not a contract — it is a convenience. The contract is
the schema attached to the tool or command in `GatewayIndex`. Always treat
`GatewayIndex` as the source of truth.

## Integration

- **SequentialThinking**: Access tool docs during multi-step analysis
- **Workflow**: Understand tool capabilities in workflow steps
- **WriteMemory**: Reference GetHelp in response hints

## Troubleshooting

**Error: "No help file found for tool: [ToolName]"**
→ The named tool/command may exist (check `GatewayIndex`) but no help asset
has been written for it. Use `GatewayIndex` with `includeSchemas: true,
includeExamples: true` for the canonical machine-readable description.

**Missing documentation for new tools**
→ Create help file in `assets/usage/tools/`, then run `UpdateAssets` with
`dryRun: false` or copy the updated `usage/` tree into the runtime assets root.
