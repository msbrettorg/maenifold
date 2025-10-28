---
layout: ../../../layouts/DocsLayout.astro
---

# FindSimilarConcepts

Find [[concept|concepts]] semantically similar to a given concept using [[vector-embeddings]] and cosine similarity. This tool discovers related concepts based on semantic meaning rather than [[graph]] co-occurrence patterns, enabling knowledge exploration through conceptual relationships beyond explicit [[WikiLink]] connections.

## When to Use This Tool

- **Semantic Discovery**: Find concepts related by meaning, not just [[graph]] connections
- **Knowledge Expansion**: Discover related concepts you haven't explicitly linked
- **Synonym Detection**: Identify concepts that mean similar things (potential [[fragmentation]])
- **Concept Exploration**: Understand the semantic neighborhood of a concept
- **Research Support**: Find related concepts for deeper investigation
- **Graph Quality**: Identify concepts that should be linked but aren't
- **Alternative Terminology**: Discover different ways people refer to similar ideas

## Key Features

- **Vector Similarity Search**: Uses [[ONNX]] [[embeddings]] (384-dimensional) for semantic comparison
- **Cosine Distance Calculation**: SQLite vector extension computes similarity scores
- **Ranked Results**: Returns concepts ordered by semantic similarity
- **Normalized Concepts**: Automatically normalizes concept names before comparison
- **Concept Exclusion**: Excludes exact matches and empty concepts from results
- **Integration Ready**: Suggests BuildContext and SearchMemories as next steps
- **Performance**: Fast vector search using indexed embeddings in database

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| conceptName | string | Yes | Concept name to find similar concepts for | "machine learning", "testing", "agent" |
| maxResults | int | No | Maximum number of similar concepts to return (default: 10) | 20 |

## Usage Examples

### Basic Similarity Search
```json
{
  "conceptName": "machine learning"
}
```
Finds 10 concepts most semantically similar to "machine learning" (e.g., [[neural-networks]], [[deep-learning]], [[AI]]).

### Extended Results
```json
{
  "conceptName": "testing",
  "maxResults": 20
}
```
Returns 20 similar concepts to discover broader semantic neighborhood.

### Exploring Agent Concepts
```json
{
  "conceptName": "agent"
}
```
Discovers agent-related concepts: [[agent-orchestration]], [[multi-agent]], [[autonomous-agent]], etc.

### Finding MCP-Related Concepts
```json
{
  "conceptName": "MCP"
}
```
Identifies MCP ecosystem concepts: [[MCP-server]], [[Model-Context-Protocol]], [[tool]], [[resource]], etc.

### Synonym Detection
```json
{
  "conceptName": "knowledge-graph"
}
```
May reveal: [[concept-graph]], [[semantic-network]], [[ontology]] - potential duplicates to consolidate.

## How It Works

### Embedding Generation
1. Normalizes input concept name (lowercase, hyphens)
2. Generates 384-dimensional [[vector-embedding]] using [[ONNX]] model
3. Converts embedding to SQLite-compatible blob format

### Similarity Calculation
1. Loads SQLite [[vector-extension]] for cosine distance calculation
2. Queries `vec_concepts` table with vector similarity search
3. Uses `vec_distance_cosine()` function to compute distances
4. Converts distance to similarity score: `similarity = 1.0 / (1.0 + distance)`
5. Ranks concepts by similarity score (higher = more similar)

### Result Filtering
- Excludes exact concept match
- Excludes empty concepts
- Limits results to maxResults parameter
- Orders by descending similarity

## Output Structure

### Successful Search
```
Similar concepts to 'machine-learning' (by semantic similarity):

  • neural-network (similarity: 0.923)
  • deep-learning (similarity: 0.891)
  • supervised-learning (similarity: 0.854)
  • artificial-intelligence (similarity: 0.832)
  • training (similarity: 0.798)
  • model (similarity: 0.776)
  • prediction (similarity: 0.743)
  • algorithm (similarity: 0.721)
  • data-science (similarity: 0.698)
  • classification (similarity: 0.673)

Suggested next steps: BuildContext, SearchMemories
```

### No Results Found
```
No similar concepts found for 'obscure-concept'. Run Sync to ensure concept embeddings are generated.
```

## Similarity Score Interpretation

### Score Ranges
- **0.90-1.00**: Extremely similar (likely synonyms or very closely related)
- **0.75-0.90**: Highly similar (strong semantic relationship)
- **0.60-0.75**: Moderately similar (related concepts)
- **0.45-0.60**: Somewhat similar (loose relationship)
- **0.30-0.45**: Weakly similar (peripheral connection)
- **< 0.30**: Barely related (may be noise)

### Use Cases by Score
- **>0.85**: Consider consolidating concepts (potential [[fragmentation]])
- **0.70-0.85**: Explore for [[WikiLink]] connections
- **0.50-0.70**: Related knowledge areas worth exploring
- **<0.50**: Distant relationships, proceed with caution

## Comparison: Semantic vs Graph Search

### FindSimilarConcepts (Semantic)
- **Basis**: Vector embeddings, semantic meaning
- **Finds**: Concepts that MEAN similar things
- **Advantage**: Discovers implicit relationships
- **Use When**: Exploring conceptual landscape, finding synonyms

### BuildContext (Graph)
- **Basis**: [[WikiLink]] co-occurrence, graph traversal
- **Finds**: Concepts explicitly linked in files
- **Advantage**: Respects your intentional connections
- **Use When**: Following your established knowledge structure

### Example Difference
Query: "testing"

**FindSimilarConcepts** might return:
- quality-assurance (similar meaning, not linked)
- validation (similar meaning, not linked)
- verification (similar meaning, not linked)

**BuildContext** might return:
- unit-test (co-occurs in same files)
- integration-test (co-occurs in same files)
- test-coverage (co-occurs in same files)

Both are valuable - semantic for discovery, graph for structure.

## Common Patterns

### Concept Quality Audit
```bash
# Find similar concepts to check for duplicates
FindSimilarConcepts conceptName="agent"

# If high-similarity concepts exist, analyze with AnalyzeConceptCorruption
AnalyzeConceptCorruption conceptFamily="agent"

# Consolidate true duplicates with RepairConcepts
RepairConcepts conceptsToReplace="agents" canonicalConcept="agent" dryRun=true
```

### Knowledge Exploration Workflow
```bash
# Step 1: Find semantically similar concepts
FindSimilarConcepts conceptName="machine-learning" maxResults=20

# Step 2: Build graph context for most similar
BuildContext conceptName="neural-network" depth=2

# Step 3: Search files using both concepts
SearchMemories query="machine learning neural network"
```

### Research Support Pattern
```bash
# Exploring a new research area
FindSimilarConcepts conceptName="graph-neural-networks"

# Each similar concept becomes a research thread
# Use SearchMemories to find existing knowledge on each
```

### Missing Link Detection
```bash
# Find concepts that should be linked but aren't
FindSimilarConcepts conceptName="testing"

# For high-similarity concepts without graph connections,
# consider adding WikiLinks to create explicit relationships
```

## Related Tools

- **BuildContext**: Explore [[concept]] relationships via [[graph]] traversal (complements semantic search)
- **SearchMemories**: Find files containing similar concepts discovered here
- **Sync**: MUST run first to generate concept embeddings in database
- **AnalyzeConceptCorruption**: Identify concept duplicates revealed by high similarity scores
- **RepairConcepts**: Consolidate semantically similar concepts that are true duplicates
- **Visualize**: See [[graph]] structure of similar concepts

## Troubleshooting

### Error: "No similar concepts found. Run Sync to ensure concept embeddings are generated"
**Cause**: vec_concepts table doesn't have embeddings for concepts
**Solution**: Run Sync tool to scan memory files and generate concept embeddings

### Error: "Unable to generate embedding for '{concept}'"
**Cause**: ONNX embedding model failed to generate vector for input
**Solution**: Check concept name is valid text, verify ONNX model is loaded correctly

### Error: "Ensure vector extension is available"
**Cause**: SQLite vector extension not loaded
**Solution**: Verify vector extension installation, check Config.DatabasePath

### Result: Only low-similarity matches
**Cause**: Concept is unique or very specific, no semantically close concepts exist
**Solution**: This is valid - not all concepts have close semantic neighbors

### Results Seem Wrong or Unexpected
**Cause**: Vector embeddings capture semantic meaning, not domain-specific relationships
**Solution**: Remember this is general language semantics, not your specific knowledge structure - use BuildContext for your intentional connections

### Same Concepts with Different Casing
**Cause**: Concepts not normalized consistently in source files
**Solution**: Use RepairConcepts to standardize casing after identifying duplicates

## Integration Workflows

### With AnalyzeConceptCorruption
```bash
# Discover potential duplicates
FindSimilarConcepts conceptName="tool" maxResults=50

# Analyze the concept family
AnalyzeConceptCorruption conceptFamily="tool"

# Consolidate true duplicates
RepairConcepts conceptsToReplace="tools,Tools" canonicalConcept="tool"
```

### With BuildContext
```bash
# Find semantically similar concepts
FindSimilarConcepts conceptName="agent"

# For interesting similar concepts, explore graph relationships
BuildContext conceptName="autonomous-agent" depth=2

# Compare semantic similarity vs explicit graph connections
```

### With SearchMemories
```bash
# Find similar concepts
FindSimilarConcepts conceptName="testing"

# Search for files using top similar concepts
SearchMemories query="testing quality-assurance validation"

# Discover knowledge using semantic neighborhood
```

## Performance Considerations

### First Run Performance
- **ONNX Model Loading**: ~400ms initial overhead for embedding generation
- **Vector Extension**: Loads automatically on first query
- **Subsequent Queries**: Much faster (~50-100ms) after initialization

### Database Requirements
- **Sync Must Run First**: Concepts need embeddings in vec_concepts table
- **Index Usage**: Vector similarity search uses specialized indices
- **Memory Usage**: 384-dimensional floats per concept (moderate overhead)

### Optimization Tips
- Use reasonable maxResults (10-20 typical, 50+ expensive)
- Run Sync periodically to keep embeddings current
- Consider caching frequent queries at application level

## Example Exploration Session

### Step 1: Start with Known Concept
```json
{
  "conceptName": "sequential-thinking",
  "maxResults": 10
}
```

### Step 2: Review Results
```
  • chain-of-thought (similarity: 0.912)
  • reasoning (similarity: 0.867)
  • workflow (similarity: 0.834)
  • thinking (similarity: 0.801)
  • analysis (similarity: 0.776)
```

### Step 3: Explore Interesting Similar Concept
```json
{
  "conceptName": "chain-of-thought",
  "maxResults": 10
}
```

### Step 4: Build Graph Context
```json
{
  "conceptName": "chain-of-thought",
  "depth": 2
}
```

### Step 5: Search Files
```json
{
  "query": "chain of thought reasoning"
}
```

## Ma Protocol Compliance

FindSimilarConcepts follows Maenifold's Ma Protocol principles:
- **Real Mathematics**: Uses actual vector cosine similarity, no heuristics
- **Transparent Scores**: Returns explicit similarity values, not opaque "relevance"
- **No Magic**: Direct SQLite vector queries with standard operations
- **Single Responsibility**: Finds similar concepts, nothing more
- **Performance Aware**: Efficient indexed vector search
- **Database Dependency**: Requires Sync to generate embeddings (explicit prerequisite)

This tool embodies Ma Protocol's principle of **space for semantic discovery** - revealing conceptual relationships that emerge from meaning itself, not just explicit connections, while maintaining transparency about how similarity is calculated.
