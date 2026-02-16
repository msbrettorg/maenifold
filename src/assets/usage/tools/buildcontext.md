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
  "communityId": 3,
  "directRelations": [
    {
      "name": "neural-networks",
      "coOccurrenceCount": 15,
      "communityId": 3,
      "files": ["memory://research/deep-learning.md"],
      "contentPreview": {
        "memory://research/deep-learning.md": "Short preview text…"
      }
    }
  ],
  "expandedRelations": ["deep-learning", "backpropagation"],
  "communitySiblings": [
    {
      "name": "gradient-descent",
      "communityId": 3,
      "sharedNeighborCount": 5,
      "normalizedOverlap": 0.62
    }
  ]
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

## Decay Weighting

Concept relationships are weighted by recency:

- Each `RelatedConcept` includes `DecayWeight` (average across source files)
- `WeightedScore = CoOccurrenceCount * DecayWeight`
- Results are sorted by `WeightedScore`, not raw co-occurrence

This means recent concept connections rank higher than stale ones, even if the stale relationship has more historical co-occurrences.

**Note:** BuildContext does NOT update `last_accessed`. Use ReadMemory on specific files to boost their recency.

## Community Detection

When community data is available (populated by `Sync` running the Louvain algorithm), BuildContext enriches its output in two ways:

### CommunityId on RelatedConcepts

Each `RelatedConcept` in `directRelations` includes an optional `communityId` (int or null). This is the Louvain community cluster the concept belongs to. When no community data exists, the field is omitted.

### CommunitySiblings

A separate `communitySiblings` array on the result surfaces concepts in the same community as the query concept that have **no direct edge** to it. These are structurally related concepts that edge traversal misses.

Each sibling includes:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Concept name |
| `communityId` | int | Community cluster ID |
| `sharedNeighborCount` | int | Number of neighbors shared with the query concept |
| `normalizedOverlap` | double | `sum(min(query_weight, sibling_weight) for shared neighbors) / sibling_total_degree` |

Siblings are filtered and capped by configurable thresholds:

| Env Var | Default | Purpose |
|---------|---------|---------|
| `MAENIFOLD_COMMUNITY_MIN_SHARED` | 3 | Minimum shared neighbors to qualify |
| `MAENIFOLD_COMMUNITY_MIN_OVERLAP` | 0.4 | Minimum normalized overlap score |
| `MAENIFOLD_COMMUNITY_MAX_SIBLINGS` | 10 | Maximum siblings returned |

### Graceful Degradation

When no community data exists (e.g., `Sync` hasn't run, or the graph is empty), `communityId` is null on all relations and `communitySiblings` is an empty array. No errors are raised. BuildContext falls back to its pre-community behavior.

### Example with Community Data

```json
{
  "conceptName": "machine-learning",
  "depth": 2,
  "communityId": 3,
  "directRelations": [
    {
      "name": "neural-networks",
      "coOccurrenceCount": 15,
      "communityId": 3,
      "files": ["memory://research/deep-learning.md"],
      "contentPreview": {
        "memory://research/deep-learning.md": "Short preview text…"
      }
    }
  ],
  "expandedRelations": ["deep-learning", "backpropagation"],
  "communitySiblings": [
    {
      "name": "gradient-descent",
      "communityId": 3,
      "sharedNeighborCount": 5,
      "normalizedOverlap": 0.62
    }
  ]
}
```

**Note:** Community siblings are additive. Direct relations and expanded relations are returned as-is regardless of community membership. Siblings surface concepts that edge traversal misses — they don't filter or gate what edges return.

## Integration

- **Sync**: Rebuild graph before BuildContext if files changed
- **Visualize**: Generate Mermaid diagrams from results
- **SearchMemories**: Find content after discovering concepts
