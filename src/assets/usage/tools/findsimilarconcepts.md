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

## Semantic vs Graph

**FindSimilarConcepts (meaning-based)**:
- Finds concepts that mean similar things
- Discovers implicit relationships
- Use for: synonym detection, exploration

**BuildContext (connection-based)**:
- Finds concepts explicitly linked in files
- Respects intentional connections
- Use for: following established structure

## Integration

- **Sync**: MUST run first to generate embeddings
- **AnalyzeConceptCorruption**: Identify duplicates from high-similarity results
- **RepairConcepts**: Consolidate similar concepts that are true duplicates
- **BuildContext**: Explore graph relationships of similar concepts
- **SearchMemories**: Find files containing discovered concepts
