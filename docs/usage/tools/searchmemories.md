# SearchMemories

Search memory FILES by content to discover your [[WikiLinks]] and knowledge. This tool provides full-text search across your maenifold knowledge base with intelligent scoring, snippet extraction, and flexible filtering options. Essential for knowledge discovery before creating new content or finding existing solutions.

## When to Use This Tool

- Finding existing knowledge before creating new memory files to avoid duplication
- Discovering related concepts, solutions, or research across your knowledge base
- Locating specific information when you remember partial content but not exact titles
- Exploring knowledge areas by searching for concept names or technical terms
- Filtering knowledge by folders, tags, or content patterns for focused discovery
- Building context for current work by finding related prior knowledge
- Verifying what knowledge already exists before starting new research or analysis
- Supporting Sequential Thinking or Workflow sessions with relevant background information

## Key Features

- **Three Search Modes**: Hybrid (default), Semantic (vector), and FullText (keyword)
- **Full-Text Content Search**: Keyword search over file content with simple scoring
- **Semantic Search**: Vector similarity search over embedded content (requires Sync)
- **Intelligent Scoring**: Prioritizes title matches and weighs term frequency for relevance
- **Smart Snippet Extraction**: Shows contextual text snippets around search terms
- **Folder-Based Filtering**: Limit searches to specific knowledge areas or projects
- **Tag-Based Filtering**: Find files matching specific categorical tags
- **Pagination Support**: Handle large result sets with page/pageSize controls
- **Automatic URI Generation**: Results include memory:// URIs for immediate ReadMemory access
- **Case-Insensitive Search**: Flexible matching regardless of capitalization
- **Multi-Term Support**: Searches for multiple terms with combined scoring
- **Concept Discovery**: Find files containing specific [[WikiLink]] concepts

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| query | string | Yes | Search terms - looks in FILE contents | "neural networks transformer" |
| mode | string | No | "Hybrid" (default), "Semantic", or "FullText" | "Hybrid" |
| pageSize | int | No | Max FILES to return (default: 10) | 15 |
| page | int | No | Page number for results (default: 1) | 2 |
| folder | string | No | Filter by folder path | "research/ai" |
| tags | string[] | No | Filter by tags (must match all) | ["machine-learning", "research"] |
| minScore | number | No | Minimum score threshold (default: 0.0). Semantics depend on mode. | 0.25 |

## Usage Examples

### Basic Content Search
```json
{
  "query": "GraphRAG implementation",
  "pageSize": 10
}
```
Finds all files containing "GraphRAG" and "implementation" with relevance scoring and snippet extraction.

### Focused Research Search
```json
{
  "query": "transformer attention mechanism",
  "folder": "research/deep-learning",
  "pageSize": 15
}
```
Searches only within the deep-learning research folder for transformer-related content.

### Tag-Based Knowledge Discovery
```json
{
  "query": "optimization performance",
  "tags": ["csharp", "performance"],
  "pageSize": 8
}
```
Finds performance optimization content specifically tagged for C# development.

### Concept Exploration
```json
{
  "query": "[[Neural Networks]]",
  "pageSize": 20
}
```
Discovers all files containing the specific Neural Networks concept with WikiLink formatting.

### Paginated Results
```json
{
  "query": "machine learning",
  "pageSize": 5,
  "page": 3
}
```
Retrieves the third page of machine learning search results with 5 files per page.

### Broad Knowledge Survey
```json
{
  "query": "debugging troubleshooting",
  "folder": "solutions"
}
```
Explores all problem-solving content in the solutions folder.

## Common Patterns

### Pre-Creation Knowledge Check
Before using WriteMemory, search for existing knowledge to avoid duplication and build on existing concepts.

### Research Context Building  
Search for background knowledge before starting new analysis or Sequential Thinking sessions.

### Solution Discovery
Search for "error", "fix", "solution", or specific error messages to find documented solutions.

### Concept Relationship Exploration
Search for specific [[WikiLink]] names to find all files that reference or discuss that concept.

### Project-Specific Knowledge Access
Use folder filtering to search within specific project or research areas.

### Tag-Based Content Curation
Combine tag filtering with content search for highly targeted knowledge discovery.

## Related Tools

- **ReadMemory**: Access full content of files found through SearchMemories using memory:// URIs
- **WriteMemory**: Create new knowledge files after checking SearchMemories for existing content
- **BuildContext**: Explore concept relationships from search results using [[WikiLink]] names
- **EditMemory**: Modify files discovered through search while maintaining [[WikiLink]] requirements
- **Sync**: Rebuild knowledge graph when search results seem incomplete or outdated
- **SequentialThinking**: Use search results to inform thinking sessions with relevant background knowledge

## Search Strategy Guide

### Effective Query Construction
- **Specific Terms**: Use precise technical terms like "async await" rather than "asynchronous"
- **Multiple Keywords**: Combine related terms like "neural networks deep learning" for broader matches
- **Concept Names**: Search for "[[Concept Name]]" to find all references to specific graph nodes
- **Problem Descriptions**: Use error messages or symptom descriptions to find solutions

### Result Interpretation
- **Score Values**: Higher scores indicate more relevant matches (title matches get +5 bonus)
- **Snippet Context**: Read snippets to understand relevance before using ReadMemory for full content
- **URI Access**: Use memory:// URIs directly with ReadMemory for immediate access

### Search Refinement
- **Too Many Results**: Add more specific terms, use folder filtering, or tag filtering
- **Too Few Results**: Reduce search terms, try synonyms, or broaden to parent folders
- **Wrong Context**: Use folder filtering to focus on specific knowledge areas

## Troubleshooting

### Error: "Query contains no informative keywords"
**Cause**: The query is empty or only contains stopwords (e.g. "the", "and", "of").  
**Solution**: Add at least one specific keyword.

### "No memories found in specified folder"
**Cause**: The folder path doesn't exist under the memory root.  
**Solution**: Fix the folder path or omit the folder filter.

### Pagination gotchas
The tool does not currently validate `page`/`pageSize` bounds. Use `page >= 1` and `pageSize >= 1`.

### minScore filters everything
**Cause**: Values outside the typical 0–1 range (e.g. 2.0) will filter out all results.  
**Solution**: Use values between 0.0 and 1.0.

### No Results Found
**Cause**: Search terms don't match existing content or folder doesn't exist  
**Solution**: Try broader terms, check folder path spelling, or search without folder filtering

### Results Don't Match Expectations  
**Cause**: Search terms may be too generic or specific concepts use different terminology  
**Solution**: Try related terms, search for [[WikiLink]] names, or use tag filtering for categorical results

### Pagination Errors
**Cause**: Requesting page beyond available results  
**Solution**: Check total results count in response and adjust page parameter accordingly

### Missing Recent Files
**Cause**: Knowledge graph may need rebuilding after recent WriteMemory operations  
**Solution**: Run Sync tool to update concept extraction and ensure all files are indexed

### Tag Filtering No Results
**Cause**: All specified tags must be present on files (AND logic, not OR)  
**Solution**: Reduce tag requirements or search each tag separately to understand content distribution

### Score Seems Low for Relevant Content
**Cause**: Search terms may not appear frequently or in titles  
**Solution**: Try searching for [[WikiLink]] names or key phrases that likely appear in titles

## Search Scoring Algorithm

SearchMemories scoring depends on `mode`:

- **FullText**: counts occurrences of each non-stopword term in content (+5 bonus if term appears in title). The raw score is normalized to 0–1 relative to the top hit.
- **Semantic**: uses vector cosine distance over embedded content and normalizes results to 0–1.
- **Hybrid**: fuses text and semantic rankings using Reciprocal Rank Fusion (RRF). `minScore` applies to the fused RRF score.

## Performance Characteristics

- **FullText** reads markdown files from disk and skips malformed files.
- **Semantic/Hybrid** use the SQLite vector extension and require embeddings generated by `Sync`.
- If vector search fails (e.g., missing extension), Hybrid still returns text results (semantic results may be empty).

## Knowledge Graph Integration

SearchMemories works seamlessly with maenifold's knowledge graph:
- **Concept Discovery**: Find files containing specific [[WikiLink]] concepts
- **Relationship Exploration**: Search results can be used with BuildContext for graph traversal
- **URI Compatibility**: Results provide memory:// URIs compatible with all memory tools
- **Graph Sync**: Use Sync after finding outdated results to refresh concept extraction

This tool serves as the primary entry point for knowledge discovery in your maenifold system, enabling efficient exploration of accumulated knowledge before creation of new content or analysis.

## Ma Protocol Compliance

SearchMemories follows maenifold's Ma Protocol principles:
- **Simplicity**: Direct file system search without complex indexing
- **No Magic**: Transparent scoring algorithm and real file access
- **Minimal Complexity**: Static methods with clear search logic
- **Real Testing**: End-to-end search validation with actual file content
- **Graceful Degradation**: Continues operation even with malformed files

The tool provides reliable knowledge discovery without unnecessary complexity, supporting the maenifold philosophy of augmented intelligence through persistent, searchable knowledge.