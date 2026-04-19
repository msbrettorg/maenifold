# Visualize

Generates Mermaid diagrams of concept relationships from `~/maenifold/memory.db` graph database.

## Parameters

- `conceptName` (string, required): Central concept (NOT a file). Example: `"machine-learning"`
- `depth` (int, optional): Graph traversal depth, 1-5 (default: 2)
- `maxNodes` (int, optional): Max nodes in diagram, 5-100 (default: 30)
- `format` (string, optional): Output mode — `"mermaid"` (default) or `"graph"`

## Returns

```
graph TD
    machine_learning -->|12| neural_networks
    machine_learning -->|8| deep_learning
    neural_networks -->|6| backpropagation
```

`format="mermaid"` returns Mermaid syntax ready for rendering. Edge weights show co-occurrence counts.

`format="graph"` returns structured JSON for graph UIs:

```json
{
  "center": "machine-learning",
  "nodes": [
    { "id": "machine-learning", "label": "machine-learning", "depth": 0 },
    { "id": "neural-networks", "label": "neural-networks", "depth": 1 }
  ],
  "edges": [
    { "source": "machine-learning", "target": "neural-networks", "weight": 12 }
  ]
}
```

## Example

```json
{
  "conceptName": "GraphRAG",
  "depth": 1,
  "maxNodes": 15,
  "format": "graph"
}
```

## Constraints

- **Depth limits**: 1-5 hops. Higher values = more connections, slower queries
- **Node limits**: 5-100 nodes. Results ordered by co-occurrence strength
- **Sync required**: Run `Sync` first or returns "CONCEPT not found" error
- **Concept normalization**: Input normalized to lowercase-with-hyphens internally

## Integration

- **BuildContext**: Textual relationship data
- **Sync**: Prerequisite - extracts `[[WikiLinks]]` like [[machine-learning]], [[research]] into graph
- **SearchMemories**: Find files containing visualized concepts
- **Obsidian/GitHub**: Render Mermaid output directly
