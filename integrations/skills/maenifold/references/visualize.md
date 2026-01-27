
# Visualize

Generates Mermaid diagrams of concept relationships from `~/maenifold/memory.db` graph database.

## Parameters

- `conceptName` (string, required): Central concept (NOT a file). Example: `"machine-learning"`
- `depth` (int, optional): Graph traversal depth, 1-5 (default: 2)
- `maxNodes` (int, optional): Max nodes in diagram, 5-100 (default: 30)

## Returns

```
graph TD
    machine_learning -->|12| neural_networks
    machine_learning -->|8| deep_learning
    neural_networks -->|6| backpropagation
```

Mermaid syntax ready for rendering. Edge weights show co-occurrence counts.

## Example

```json
{
  "conceptName": "GraphRAG",
  "depth": 1,
  "maxNodes": 15
}
```

## Constraints

- **Depth limits**: 1-5 hops. Higher values = more connections, slower queries
- **Node limits**: 5-100 nodes. Results ordered by co-occurrence strength
- **Sync required**: Run `Sync` first or returns "CONCEPT not found" error
- **Concept normalization**: Input normalized to lowercase-with-hyphens internally

## Integration

- **BuildContext**: Textual relationship data
- **Sync**: Prerequisite - extracts `[[concepts]]` into graph
- **SearchMemories**: Find files containing visualized concepts
- **Obsidian/GitHub**: Render Mermaid output directly
