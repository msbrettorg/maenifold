---
layout: ../../../layouts/DocsLayout.astro
---

# WriteMemory

Create new knowledge files in the Maenifold memory system with automatic knowledge graph integration. This tool builds your persistent knowledge base through [[WikiLink]] concepts that automatically connect related information. REQUIRES at least one [[concept]] in double brackets to ensure knowledge graph connectivity.

## When to Use This Tool

- Creating new knowledge files with structured information
- Capturing insights, research, or analysis that should persist across sessions  
- Starting new knowledge areas or topics with proper graph connections
- Documenting solutions, patterns, or discoveries for future reference
- Building interconnected knowledge networks through concept relationships
- Storing structured information that will be searched or referenced later
- Creating memory files that integrate with Sequential Thinking or Workflow sessions

## Key Features

- **Automatic Knowledge Graph Integration**: Every [[concept]] becomes a searchable graph node
- **WikiLink Enforcement**: Validates content contains at least one [[concept]] for connectivity
- **Flexible Organization**: Optional folder structure for logical grouping
- **Tagging System**: Add categorical tags for enhanced discoverability  
- **Ma Protocol Compliance**: Files stay under 250 lines, encouraging focused content
- **Automatic Metadata**: Generates timestamps, permalinks, and file structure
- **Checksum Generation**: Provides content verification for safe concurrent editing
- **URI-based Access**: Creates memory:// URIs for consistent file references

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| title | string | Yes | Human-readable title for the memory file | "Machine Learning Fundamentals" |
| content | string | Yes | Main content with required [[concepts]] in brackets | "Learning about [[Neural Networks]] and [[Deep Learning]] applications in [[Computer Vision]]" |
| folder | string | No | Optional folder path for organization | "research/ai" or "projects/2024" |
| tags | string[] | No | Optional tags for categorization | ["ai", "research", "fundamentals"] |

## Usage Examples

### Basic Knowledge Creation
```json
{
  "title": "GraphRAG Implementation Notes",
  "content": "Implementing [[GraphRAG]] requires understanding [[Vector Embeddings]] and [[Knowledge Graphs]]. Key insight: combine [[Semantic Search]] with [[Graph Traversal]] for enhanced context retrieval."
}
```
Creates a basic memory file with automatic concept extraction and graph integration.

### Organized Research File
```json
{
  "title": "Transformer Architecture Deep Dive",
  "content": "The [[Transformer]] architecture revolutionized [[Natural Language Processing]] through [[Self-Attention]] mechanisms. Key components include [[Multi-Head Attention]], [[Positional Encoding]], and [[Layer Normalization]].\n\n## Key Insights\n- [[Attention]] allows parallel processing unlike [[RNN]] sequential approach\n- [[BERT]] and [[GPT]] are both [[Transformer]]-based but use different training objectives",
  "folder": "research/deep-learning",
  "tags": ["transformers", "nlp", "architecture", "research"]
}
```
Creates an organized research file with folder structure and comprehensive tagging.

### Problem-Solution Documentation
```json
{
  "title": "Debugging Memory Leaks in C# Applications", 
  "content": "Encountered [[Memory Leak]] in [[C# Application]] during [[Performance Testing]]. Root cause: [[Event Handlers]] not properly unsubscribed in [[Observer Pattern]] implementation.\n\n## Solution\n- Use [[Weak References]] for event subscriptions\n- Implement [[IDisposable]] pattern properly\n- Apply [[RAII]] principles from [[Resource Management]]",
  "folder": "solutions/performance",
  "tags": ["debugging", "performance", "csharp", "memory-management"]
}
```
Documents a specific problem-solution pattern with technical concepts.

## Common Patterns

### Research and Learning Notes
Use WriteMemory to capture learning insights with concept connections. Every new concept becomes searchable and linkable to future knowledge.

### Project Documentation  
Create project-specific memory files in dedicated folders. Use consistent tagging schemes for project phases or components.

### Solution Cataloging
Document solutions to technical problems with [[concept]] links to technologies, patterns, and methodologies used.

### Sequential Thinking Integration
WriteMemory works seamlessly with Sequential Thinking sessions - capture thinking results as persistent knowledge for future reference.

### Knowledge Graph Building
Each [[concept]] becomes a node. Related files sharing concepts automatically connect, building your knowledge graph organically.

## Related Tools

- **SearchMemories**: Find existing knowledge by content or concepts before creating new files
- **ReadMemory**: Access created files by URI or title for reference or editing
- **EditMemory**: Modify existing files while maintaining [[concept]] requirements
- **Sync**: Rebuild knowledge graph after creating multiple files
- **BuildContext**: Explore concept relationships from files you've created
- **SequentialThinking**: Reference memory files in thinking sessions and capture results back to memory

## Folder Organization Patterns

### Recommended Folder Structure
```
research/           # Research notes and findings
projects/           # Project-specific documentation  
solutions/          # Problem-solution pairs
methodologies/      # Process and methodology documentation
chain-of-thought/   # Automatic from thinking sessions
concepts/           # Pure concept definitions
```

### Folder Best Practices
- Use hierarchical paths like `research/ai/transformers` for deep organization
- Keep folder names simple and descriptive
- Consider using dates in folders like `projects/2024/q4` for temporal organization
- Match folder structure to your workflow and thinking patterns

## Troubleshooting

### Error: "Content must contain at least one [[concept]]"
**Cause**: Content lacks [[WikiLink]] concepts in double brackets  
**Solution**: Add relevant concepts like `[[Machine Learning]]` or `[[Software Architecture]]` to connect your knowledge to the graph

### Error: "Memory file not found during read"
**Cause**: File title or URI reference is incorrect  
**Solution**: Use exact title from WriteMemory response or full memory:// URI format

### Warning: Creating orphaned knowledge
**Cause**: Using very unique [[concepts]] that don't connect to existing knowledge  
**Solution**: Include some established concepts alongside new ones to maintain graph connectivity  

### File organization issues
**Cause**: Inconsistent folder naming or deep nesting  
**Solution**: Use consistent folder patterns and avoid more than 3-4 levels of nesting

### Content too long (Ma Protocol violation)
**Cause**: Single file exceeds 250 lines  
**Solution**: Split into multiple focused files with shared [[concepts]] for connection

## Knowledge Graph Integration

Every WriteMemory operation automatically:
1. **Extracts [[concepts]]** from content using WikiLink parsing
2. **Creates graph nodes** for new concepts or connects to existing ones  
3. **Builds relationships** between concepts within the same file
4. **Enables traversal** through BuildContext and Visualize tools
5. **Powers search** through concept-based queries in SearchMemories

The [[concept]] requirement ensures no orphaned knowledge - every file connects to your growing knowledge graph, making information discoverable and relationship-aware.

## Ma Protocol Compliance

WriteMemory follows Maenifold's Ma Protocol principles:
- **Simplicity**: Single responsibility for knowledge creation
- **No Magic**: Real file system storage with transparent metadata
- **Minimal Complexity**: Static methods, clear parameters, direct operation
- **Real Testing**: End-to-end file creation and validation
- **250-line Limit**: Encourages focused, coherent knowledge units

This tool creates the foundation for your Maenifold knowledge system - every file becomes part of your growing cognitive infrastructure.