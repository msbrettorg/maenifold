# EditMemory

Modifies existing memory files with [[WikiLink]] preservation and checksum safety for graph integrity.

## Parameters

- `identifier` (string, required): Memory URI or title. Example: `"memory://research/notes"` or `"notes"`
- `operation` (string, required): Edit type: `"append"`, `"prepend"`, `"find_replace"`, `"replace_section"`
- `content` (string, required): Content to add/replace. MUST contain at least one `[[concept]]`.
- `checksum` (string, optional): From ReadMemory. Prevents stale edits.
- `findText` (string, optional): For find_replace. Text to find.
- `expectedCount` (int, optional): For find_replace. Expected match count.
- `sectionName` (string, optional): For replace_section. Markdown header name.

## Returns

```json
{
  "uri": "memory://research/notes",
  "checksum": "xyz789...",
  "operation": "append",
  "modified": "2025-11-03T10:30:00Z"
}
```

## Operations

### append
Adds content to file end.
```json
{
  "identifier": "research-notes",
  "operation": "append",
  "content": "## Latest\n\nNew insights on [[Neural Networks]] and [[Attention Mechanisms]]."
}
```

### prepend
Adds content to file start.
```json
{
  "identifier": "research-notes",
  "operation": "prepend",
  "content": "# Prerequisites\n\nRequires [[Statistics]] knowledge.\n\n"
}
```

### find_replace
Replaces all occurrences.
```json
{
  "identifier": "research-notes",
  "operation": "find_replace",
  "findText": "ML algorithms",
  "content": "[[Machine Learning]] algorithms",
  "expectedCount": 5,
  "checksum": "abc123..."
}
```

### replace_section
Replaces markdown section by header.
```json
{
  "identifier": "project-status",
  "operation": "replace_section",
  "sectionName": "Current Status",
  "content": "Completed. Key insights: [[Agile Methodology]] and [[CI/CD]] improvements."
}
```

## Workflow

1. **ReadMemory** with `includeChecksum=true`
2. **EditMemory** with checksum from step 1
3. **Sync** if bulk edits (rebuilds graph)

## Constraints

- **[[concept]] required**: New content MUST contain at least one `[[WikiLink]]`
- **Checksum validation**: Detects conflicts between read and edit
- **Section names**: Must match exact markdown header text (case-sensitive)
- **expectedCount**: Validates find_replace safety

## Integration

- **ReadMemory**: Get checksum before editing
- **SearchMemories**: Find files to update
- **Sync**: Rebuild graph after bulk edits
- **BuildContext**: Verify concept connections after edits
