---
name: ma:readmemory
description: Retrieves content from memory files by URI or title with checksum for safe editing
---

# ReadMemory

Retrieves content from `~/maenifold/memory/` files by URI or title with checksum for safe editing.

## Parameters

- `identifier` (string, required): `memory://uri` or file title. Example: `"memory://projects/ai-research"` or `"AI Research Notes"`
- `includeChecksum` (bool, optional): Return checksum for edit safety. Default: `true`

## Returns

```json
{
  "title": "Machine Learning Fundamentals",
  "uri": "memory://research/ai/machine-learning-fundamentals",
  "location": "research/ai/machine-learning-fundamentals.md",
  "created": "2024-01-15 14:30:00",
  "modified": "2024-01-20 09:15:00",
  "checksum": "abc123...",
  "content": "# Machine Learning Fundamentals\n\nImplementing [[Machine Learning]] requires..."
}
```

## Example

```json
{
  "identifier": "memory://projects/machine-learning",
  "includeChecksum": true
}
```

## Lookup Methods

- **URI lookup**: Direct access via `memory://path/file` (faster)
- **Title lookup**: Searches by slugified title (case-insensitive, folder-agnostic)

## Integration

- **EditMemory**: Always read first to get checksum for safe editing
- **SearchMemories**: Find files when identifier unknown
- **BuildContext**: Explore concept relationships from retrieved content
- **ExtractConceptsFromFile**: Analyze `[[WikiLinks]]` in file
