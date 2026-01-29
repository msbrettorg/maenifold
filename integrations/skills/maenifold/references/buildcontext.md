
# BuildContext

Traverses concept relationships via multi-hop graph queries.

## Parameters

- `conceptName` (string, required): CONCEPT name (NOT a file). Example: `"Machine Learning"`
- `depth` (int, optional): Graph hops (default: 2). Range: 0+ (negative values error)
- `maxEntities` (int, optional): Max related entities to return (default: 20)
- `includeContent` (bool, optional): Include short content previews for source files (default: false)

## Returns

```json
{
  "conceptName": "machine-learning",
  "depth": 2,
  "directRelations": [
    {
      "name": "neural-networks",
      "coOccurrenceCount": 15,
      "files": ["memory://research/deep-learning.md"],
      "contentPreview": {
        "memory://research/deep-learning.md": "Short preview text…"
      }
    }
  ],
  "expandedRelations": ["deep-learning", "backpropagation"]
}
```

## Example

```json
{
  "conceptName": "GraphRAG",
  "depth": 2,
  "maxEntities": 30
}
```

## Constraints

- **No hard requirement to Sync**: If the graph is empty or the concept doesn't exist yet, the tool returns an empty result (no error). Run `Sync` to populate/update the graph.
- **Depth semantics**:
  - `depth = 0` and `depth = 1` return only `directRelations`.
  - `depth > 1` also returns `expandedRelations`.
- **ExpandedRelations traversal**: Expansion starts from up to the top 5 direct relations (by co-occurrence), then traverses outward.
- **includeContent behavior**: Adds a ~200 character sentence-aware preview for up to 3 source files per direct relation; unreadable files are skipped silently.
- **WikiLink normalization**: `[[Machine Learning]]` → `machine-learning`

## Integration

- **Sync**: Rebuild graph before BuildContext if files changed
- **Visualize**: Generate Mermaid diagrams from results
- **SearchMemories**: Find content after discovering concepts
