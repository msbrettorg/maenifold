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

## Troubleshooting

**Error: "No help file found for tool: [ToolName]"**
→ Tool names are case-insensitive.

**Missing documentation for new tools**
→ Help files are managed in the maenifold source repository under `src/assets/usage/tools/` (250-line limit)
