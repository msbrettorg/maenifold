---
layout: ../../../layouts/DocsLayout.astro
---

# BuildContext

Explore and traverse concept relationships in your Maenifold knowledge graph through intelligent graph navigation. This tool discovers connections between [[concepts]] by analyzing co-occurrence patterns in your memory files, building contextual understanding through relationship exploration and multi-hop graph traversal.

## When to Use This Tool

- Exploring concept relationships and discovering connected ideas in your knowledge graph
- Building contextual understanding around a central concept before deep analysis
- Researching topic connections to understand conceptual neighborhoods  
- Gathering related concepts for Sequential Thinking or Workflow sessions
- Discovering knowledge patterns and concept clusters in your memory system
- Finding conceptual bridges between seemingly unrelated topics
- Preparing comprehensive context for complex problem-solving or analysis tasks

## Key Features

- **Multi-Hop Graph Traversal**: Navigate 1-hop direct relationships or extend to 2+ hops for broader context discovery
- **Co-Occurrence Analysis**: Identifies concept relationships based on shared appearances in memory files
- **Intelligent Relationship Ranking**: Orders related concepts by co-occurrence frequency and relevance
- **Source File Tracking**: Shows which memory files contain concept relationships for verification
- **Scalable Entity Control**: Configure maximum entities to focus exploration or cast wider nets
- **Concept Normalization**: Automatically handles concept name variations and formatting
- **Graph Existence Validation**: Confirms concepts exist in your knowledge graph before traversal
- **Contextual File References**: Links back to original memory files containing concept relationships

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| conceptName | string | Yes | CONCEPT name to build context around (NOT a file!) | "Machine Learning" |
| depth | int | No | How many hops in the CONCEPT GRAPH (default: 2) | 3 |
| maxEntities | int | No | Max entities to return (default: 20) | 50 |
| includeContent | bool | No | Include full content (default: false) | true |

## Usage Examples

### Basic Concept Exploration
```json
{
  "conceptName": "Neural Networks"
}
```
Discovers direct relationships to Neural Networks concept with default 2-hop depth and 20 entity limit.

### Focused Direct Relationships
```json
{
  "conceptName": "GraphRAG",
  "depth": 1,
  "maxEntities": 10
}
```
Explores only immediate (1-hop) relationships to GraphRAG, limiting results to 10 most relevant concepts.

### Deep Context Building
```json
{
  "conceptName": "Transformer Architecture", 
  "depth": 3,
  "maxEntities": 50
}
```
Performs deep 3-hop traversal around Transformer Architecture, returning up to 50 related concepts for comprehensive context.

### Research Preparation
```json
{
  "conceptName": "Memory Systems",
  "depth": 2,
  "maxEntities": 30,
  "includeContent": true
}
```
Builds research context around Memory Systems with 2-hop exploration, returning 30 entities with full content for detailed analysis.

## Common Patterns

### Research Context Building
Use BuildContext before starting research to understand the conceptual landscape. Discover related topics, competing approaches, and knowledge gaps in your current understanding.

### Sequential Thinking Preparation  
Build context around key concepts before Sequential Thinking sessions to prime your analysis with related ideas and connection patterns.

### Knowledge Gap Discovery
Explore concept neighborhoods to identify missing relationships or underdeveloped areas in your knowledge graph.

### Topic Bridge Finding
Use multi-hop traversal to discover conceptual bridges between different domains, revealing unexpected connections in your knowledge.

### Workflow Context Gathering
Build comprehensive context before structured workflows like design thinking or problem solving to ensure rich conceptual input.

### Concept Clustering Analysis
Analyze how concepts cluster together through co-occurrence patterns, revealing natural knowledge organization in your memory system.

## Related Tools

- **Sync**: Must run Sync before BuildContext to ensure graph contains latest concept relationships from memory files
- **SearchMemories**: Use to find specific content after BuildContext reveals related concepts
- **Visualize**: Generate Mermaid diagrams of concept relationships discovered through BuildContext exploration
- **SequentialThinking**: Use BuildContext results to prime thinking sessions with related concepts and connections
- **WriteMemory**: Create new memory files using concepts discovered through BuildContext exploration
- **ReadMemory**: Access specific memory files referenced in BuildContext source file listings

## Graph Traversal Patterns

### 1-Hop Direct Relationships
Perfect for understanding immediate concept connections and finding most relevant related ideas.
```json
{"conceptName": "Deep Learning", "depth": 1}
```

### 2-Hop Extended Context  
Ideal balance of breadth and focus, discovering concept neighborhoods while maintaining relevance.
```json
{"conceptName": "Attention Mechanism", "depth": 2}
```

### 3+ Hop Broad Exploration
Use for discovering distant relationships and finding unexpected conceptual bridges across domains.
```json
{"conceptName": "Optimization", "depth": 3, "maxEntities": 40}
```

### Focused vs. Broad Entity Limits
- **Focused (5-15 entities)**: Core relationships only, highly relevant concepts
- **Balanced (20-30 entities)**: Good coverage with manageable scope  
- **Broad (40+ entities)**: Comprehensive exploration, discovering edge relationships

## Troubleshooting

### Error: "CONCEPT 'X' not found in graph"
**Cause**: The concept doesn't exist in your knowledge graph or needs different spelling  
**Solution**: Run Sync to update graph from recent memory files, or check concept spelling and capitalization

### Error: "Run sync first"
**Cause**: Knowledge graph is empty or outdated compared to memory files  
**Solution**: Execute Sync tool to extract concepts from memory files and build/update the graph database

### No relationships found for existing concept
**Cause**: Concept exists but appears in isolation without co-occurring with other [[concepts]]  
**Solution**: Add the concept to existing memory files alongside related concepts to build relationships

### Traversal depth returns limited results
**Cause**: Your knowledge graph has sparse connections or concept clusters are isolated  
**Solution**: Increase maxEntities parameter or create memory files that bridge concept areas with shared [[concepts]]

### Performance issues with high depth/entity limits
**Cause**: Very dense concept graphs with high connectivity can create large result sets  
**Solution**: Reduce depth to 1-2 or lower maxEntities to 20-30 for faster traversal

### Missing expected concept relationships
**Cause**: Related concepts may exist in different memory files without co-occurrence  
**Solution**: Create bridging memory files that mention related concepts together to establish graph connections

## Graph Theory Foundation

BuildContext implements **LazyGraphRAG** principles through:

### Co-Occurrence Relationship Model
Concepts that appear together in the same memory file create weighted edges based on frequency of co-occurrence across multiple files.

### Breadth-First Traversal  
Explores concept relationships level by level, prioritizing closer relationships while extending to discover broader context.

### Weighted Edge Ranking
Relationships are ranked by co-occurrence count, ensuring most frequent concept pairings appear first in results.

### Source Provenance
Every relationship tracks which memory files contain the co-occurring concepts, enabling verification and deeper investigation.

## Integration with Maenifold Architecture

### Sequential Thinking Integration
Use BuildContext to gather conceptual context before thinking sessions:
```
BuildContext → SequentialThinking (with rich concept context)
```

### Workflow Preparation
Build comprehensive context before structured methodologies:
```  
BuildContext → Design Thinking Workflow (with related concepts)
```

### Knowledge Graph Evolution
As you create more memory files with [[concepts]], BuildContext discoveries become richer and more connected.

### Memory System Synergy
BuildContext reveals the structure of your knowledge while SearchMemories finds the content - complementary tools for knowledge exploration.

## Ma Protocol Compliance

BuildContext follows Maenifold's Ma Protocol principles:
- **Simplicity**: Single responsibility for graph traversal and relationship discovery
- **No Magic**: Real SQLite database with transparent concept relationship storage
- **Minimal Complexity**: Static methods with clear graph algorithm implementation
- **Real Testing**: End-to-end graph building and traversal validation
- **Performance Focus**: Efficient queries with result limits and depth control

This tool transforms your memory files into an explorable knowledge graph, revealing the hidden connections and patterns in your accumulated knowledge through intelligent relationship traversal.