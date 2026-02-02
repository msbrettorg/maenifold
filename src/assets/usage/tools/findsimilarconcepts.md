# FindSimilarConcepts

Discovers semantically similar concepts using 384-dimensional vector embeddings and cosine similarity.

## Parameters

- `conceptName` (string, required): Concept to find similar concepts for
- `maxResults` (int, optional): Maximum results to return (default: 10)

## Returns

```
Similar concepts to 'machine-learning' (by semantic similarity):

  • neural-network (similarity: 0.923)
  • deep-learning (similarity: 0.891)
  • supervised-learning (similarity: 0.854)
  • artificial-intelligence (similarity: 0.832)

Suggested next steps: BuildContext, SearchMemories
```

## Example

```json
{
  "conceptName": "agent",
  "maxResults": 20
}
```

## Similarity Scores

- **0.90-1.00**: Likely synonyms - consider consolidation
- **0.75-0.90**: Strong relationship - good for WikiLinks
- **0.60-0.75**: Related concepts - explore connections
- **< 0.60**: Weak relationship

Notes:
- `conceptName` must be non-empty; otherwise the tool returns an error.
- Scores can appear **saturated** (e.g., many results at `1.000`) depending on the current embedding distribution and vector distance values. Treat this tool primarily as a **ranking signal**, and validate via `BuildContext` / `SearchMemories`.
- Very large `maxResults` values can produce extremely long output.

## Semantic vs Graph

**FindSimilarConcepts (meaning-based)**:
- Finds concepts that mean similar things
- Discovers implicit relationships
- Use for: synonym detection, exploration

**BuildContext (connection-based)**:
- Finds concepts explicitly linked in files
- Respects intentional connections
- Use for: following established structure

## Decay Weighting

FindSimilarConcepts uses vector similarity, not file timestamps. Decay weighting does not apply to this tool directly.

However, once you identify similar concepts, use **BuildContext** or **SearchMemories** to explore them—those tools apply decay weighting to rank recent content higher.

## Integration

- **Sync**: Run first to populate `vec_concepts` embeddings (otherwise you may get few/no results)
- **AnalyzeConceptCorruption**: Identify duplicates from high-similarity results
- **RepairConcepts**: Consolidate similar concepts that are true duplicates
- **BuildContext**: Explore graph relationships of similar concepts (decay-weighted)
- **SearchMemories**: Find files containing discovered concepts (decay-weighted)
