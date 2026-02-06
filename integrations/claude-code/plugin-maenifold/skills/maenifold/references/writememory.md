
# WriteMemory

Creates knowledge files in `~/maenifold/memory/` with automatic graph integration via `[[WikiLink]]` concepts.

## Parameters

- `title` (string, required): File title. Generates URI: `memory://{folder}/{normalized-title}`
- `content` (string, required): Markdown content. MUST contain at least one `[[WikiLink]]` in double brackets.
- `folder` (string, optional): Folder path under memory root. Example: `"research/ai"`
- `tags` (string[], optional): Categorization tags. Example: `["ai", "research"]`

## Returns

```json
{
  "uri": "memory://research/ai/machine-learning-fundamentals",
  "checksum": "abc123...",
  "conceptsExtracted": ["machine-learning", "neural-networks"],
  "location": "~/maenifold/memory/research/ai/machine-learning-fundamentals.md"
}
```

## Example

```json
{
  "title": "GraphRAG Implementation Notes",
  "content": "Implementing [[GraphRAG]] requires [[Vector Embeddings]] and [[Knowledge Graphs]]. Combine [[Semantic Search]] with [[Graph Traversal]].",
  "folder": "research/ai",
  "tags": ["graphrag", "research"]
}
```

## Constraints

- **[[WikiLink]] required**: Content MUST contain at least one `[[WikiLink]]` or operation fails
- **WikiLink format**: Use `[[Concept Name]]` → normalizes to `concept-name` in graph
- **250-line limit**: Ma Protocol compliance - split large content into multiple focused files
- **Folder depth**: Recommend ≤3 levels to avoid deep nesting

## Integration

- **Sync**: Run after bulk writes to rebuild graph indices
- **SearchMemories**: Find files by content or concepts
- **BuildContext**: Traverse concept relationships
- **EditMemory**: Modify using URI/title + checksum from response
