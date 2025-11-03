# BuildContext

Traverses concept relationships via multi-hop graph queries.

## Parameters

- `conceptName` (string, required): CONCEPT name (NOT a file). Example: `"Machine Learning"`
- `depth` (int, optional): Graph hops (default: 2). Range: 1-5
- `maxEntities` (int, optional): Max results (default: 20). Range: 5-100
- `includeContent` (bool, optional): Include file content (default: false)

## Returns

```json
{
  "relatedConcepts": [
    {
      "concept": "neural-networks",
      "coOccurrenceCount": 15,
      "sourceFiles": ["memory://research/deep-learning.md"]
    }
  ],
  "totalFound": 8,
  "depth": 2
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

- **Sync first**: Error `"Run sync first"` if graph empty
- **Concept exists**: Error `"CONCEPT 'X' not found"` if missing
- **WikiLink format**: `[[Machine Learning]]` → `machine-learning`

## Integration

- **Sync**: Rebuild graph before BuildContext if files changed
- **Visualize**: Generate Mermaid diagrams from results
- **SearchMemories**: Find content after discovering concepts
