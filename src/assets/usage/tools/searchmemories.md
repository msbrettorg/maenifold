# SearchMemories

Search memory files by content with three modes: **Hybrid** (default), **Semantic**, and **FullText**.

## Parameters

- `query` (string, required): Search terms - looks in FILE contents
- `mode` (string, optional): `"Hybrid"` (default), `"Semantic"`, or `"FullText"`
- `pageSize` (int, optional): Max FILES to return (default: 10)
- `page` (int, optional): Page number for results (default: 1)
- `folder` (string, optional): Filter by folder path
- `tags` (string[], optional): Filter by tags (must match all)
- `minScore` (number, optional): Minimum score threshold (default: 0.0)

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

- **Hybrid**: Reciprocal Rank Fusion (RRF) over text + semantic ranks; `minScore` applies to the *fused* score.
- **Semantic**: Vector similarity normalized to 0–1; `minScore` applies to the semantic score.
- **FullText**: Term-frequency score normalized to 0–1 (relative to best hit); `minScore` applies to the normalized score.

Notes:
- Queries must contain at least one non-stopword keyword (empty/stopword-only queries return an error).
- The tool does not currently enforce `page >= 1` / `pageSize >= 1`; use those values anyway (e.g., `pageSize: 0` returns no results).

## Patterns

- **Pre-creation check**: Search before WriteMemory to avoid duplication
- **Context building**: Find background for Sequential Thinking sessions
- **Solution discovery**: Search error messages or "fix", "solution"
- **Concept exploration**: Search `[[WikiLink]]` names like [[machine-learning]], [[research]] for graph nodes
- **Project-specific**: Use folder filtering for targeted discovery

## Integration

- **ReadMemory**: Access full content using memory:// URIs from results
- **BuildContext**: Traverse concept relationships from search results
- **Sync**: Rebuild graph when results seem incomplete
- **WriteMemory**: Create new knowledge after checking for existing content
