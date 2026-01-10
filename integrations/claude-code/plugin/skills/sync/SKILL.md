---
name: ma:sync
description: Rebuilds knowledge graph from all WikiLink concepts in memory files
user-invocable: true
---

# Sync

Rebuilds knowledge graph from all `[[WikiLink]]` concepts in memory files. Extracts concepts, builds co-occurrence relationships, updates search indices.

## Parameters

None. Operates on entire memory system.

## Returns

```json
{
  "conceptsProcessed": 1247,
  "relationshipsBuilt": 3891,
  "filesIndexed": 234,
  "abandonedSessions": 3
}
```

## When to Use

- After bulk writes/edits to refresh graph
- Before BuildContext/Visualize to ensure current data
- When SearchMemories results seem stale
- After external file system changes
- System maintenance (cleanup abandoned sessions, optimize indices)

## Process

1. **Session Cleanup**: Mark sessions active >30min as abandoned
2. **Concept Extraction**: Extract all `[[concepts]]` from .md files, normalize (lowercase-with-hyphens)
3. **Graph Construction**: Create concept nodes, build co-occurrence edges weighted by frequency
4. **Content Indexing**: Update FTS5 index for SearchMemories

## Example

```json
{}
```

## Database Schema

- `concepts`: Normalized concept names with timestamps
- `concept_mentions`: Concept-to-file links with occurrence counts
- `concept_graph`: Weighted edges between co-occurring concepts
- `file_content`: Full text + metadata
- `file_search`: FTS5 virtual table

## Integration

- **BuildContext/Visualize**: Require synced graph data
- **SearchMemories**: Uses FTS index + concept graph
- **WriteMemory/EditMemory**: Generate content that Sync processes
- **SequentialThinking/Workflow**: Create sessions Sync monitors

## Troubleshooting

- **Database locked**: Wait for concurrent ops, retry
- **No concepts found**: Ensure files contain `[[concept]]` references
- **Slow performance**: Normal for large knowledge bases
- **Abandoned sessions warning**: Expected cleanup behavior
