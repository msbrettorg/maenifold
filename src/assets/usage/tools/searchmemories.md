# SearchMemories

Full-text content search across `~/maenifold/memory/` with scoring, snippets, and flexible filtering.

## Parameters

- `query` (string, required): Search terms - looks in FILE contents
- `pageSize` (int, optional): Max FILES to return (default: 10)
- `page` (int, optional): Page number for results (default: 1)
- `folder` (string, optional): Filter by folder path
- `tags` (string[], optional): Filter by tags (must match all)

## Returns

```json
{
  "results": [
    {
      "uri": "memory://research/ai/graphrag-notes",
      "title": "GraphRAG Implementation Notes",
      "score": 8.5,
      "snippet": "...implementing [[GraphRAG]] requires [[Vector Embeddings]]..."
    }
  ],
  "totalResults": 23,
  "page": 1,
  "pageSize": 10
}
```

## Examples

```json
// Basic search
{
  "query": "GraphRAG implementation",
  "pageSize": 10
}

// Focused research
{
  "query": "transformer attention",
  "folder": "research/deep-learning",
  "pageSize": 15
}

// Tag-based discovery
{
  "query": "optimization",
  "tags": ["csharp", "performance"]
}

// Concept exploration
{
  "query": "[[Neural Networks]]"
}
```

## Scoring

- **Content matches**: +1 per occurrence of each search term
- **Title matches**: +5 bonus for terms in titles
- **Case insensitive**: Flexible matching
- **Multi-term**: Each term contributes independently

## Patterns

- **Pre-creation check**: Search before WriteMemory to avoid duplication
- **Context building**: Find background for Sequential Thinking sessions
- **Solution discovery**: Search error messages or "fix", "solution"
- **Concept exploration**: Search `[[concept]]` names for graph nodes
- **Project-specific**: Use folder filtering for targeted discovery

## Integration

- **ReadMemory**: Access full content using memory:// URIs from results
- **BuildContext**: Traverse concept relationships from search results
- **Sync**: Rebuild graph when results seem incomplete
- **WriteMemory**: Create new knowledge after checking for existing content
