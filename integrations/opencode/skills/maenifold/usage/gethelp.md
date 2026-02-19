# GetHelp

Loads complete tool documentation from `~/maenifold/assets/usage/tools/{toolname}.md` for detailed usage guidance.

## Parameters

- `toolName` (string, required): Tool name. Example: `"WriteMemory"`, `"SearchMemories"`, `"BuildContext"`

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

## Available Tools

**Memory**: BuildContext, DeleteMemory, EditMemory, ExtractConceptsFromFile, MoveMemory, ReadMemory, SearchMemories, WriteMemory

**System**: GetConfig, GetHelp, ListMemories, MemoryStatus, RecentActivity, Sync

**Thinking**: SequentialThinking, Workflow

**Resource**: ListAssets, ReadMcpResource

**Visualization**: Visualize

## Integration

- **SequentialThinking**: Access tool docs during multi-step analysis
- **Workflow**: Understand tool capabilities in workflow steps
- **WriteMemory**: Reference GetHelp in response hints

## Troubleshooting

**Error: "No help file found for tool: [ToolName]"**
→ Check available tools list above. Tool names are case-sensitive.

**Missing documentation for new tools**
→ Create help file in `/src/assets/usage/tools/` (250-line limit, Ma Protocol compliance)
