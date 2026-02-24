# Sync

Rebuilds knowledge graph from all `[[WikiLink]]` concepts in memory files. Extracts concepts, builds co-occurrence relationships, updates search indices.

## Parameters

None. Operates on entire memory system.

## Returns

```json
{
  "conceptsProcessed": 1247,
  "relationshipsBuilt": 3891,
  "filesIndexed": 234,
  "abandonedSessions": 3,
  "communitiesDetected": 10,
  "modularity": 0.67
}
```

## When to Use

- After bulk writes/edits to refresh graph
- Before BuildContext/Visualize to ensure current data
- When SearchMemories results seem stale
- After external file system changes
- System maintenance (cleanup abandoned sessions, optimize indices)

## Process

1. **Session Cleanup**: Mark sessions active >30min as abandoned
2. **Concept Extraction**: Extract all `[[WikiLinks]]` like [[machine-learning]], [[knowledge-graph]] from .md files, normalize (lowercase-with-hyphens)
   - **Mtime optimization**: Files whose modification time matches `last_indexed` in the database are skipped without reading, making re-syncs fast for large knowledge bases (only changed files are reprocessed). When mtime differs, a size guard and MD5 hash check prevent unnecessary reprocessing if content is unchanged.
   - **Note**: Concepts listed in `memory/.wikilink-filter.json` are blocked at write time (`WriteMemory`/`EditMemory`) and never enter memory files, so they are naturally excluded from the graph.
3. **Graph Construction**: Create concept nodes, build co-occurrence edges weighted by frequency
4. **Community Detection**: Load concept graph (co_occurrence_count as edge weights) into memory, run Phase 1 Louvain modularity optimization (deterministic seed, configurable gamma via `MAENIFOLD_LOUVAIN_GAMMA`, default 1.0), persist results to `concept_communities` table (atomic DELETE+INSERT in transaction), report community count and modularity score
5. **Content Indexing**: Update FTS5 index for SearchMemories

## Example

```json
{}
```

## Database Schema

- `concepts`: Normalized concept names with timestamps
- `concept_mentions`: Concept-to-file links with occurrence counts
- `concept_graph`: Weighted edges between co-occurring concepts
- `concept_communities`: Community assignments per concept (concept_name PK with FK to concepts, community_id, modularity, resolution, timestamp)
- `file_content`: Full text + metadata
- `file_search`: FTS5 virtual table
- `vec_concepts`: Vector table for concept embeddings (384-dim, all-MiniLM-L6-v2 via ONNX)
- `vec_memory_files`: Vector table for memory file embeddings (384-dim)

## Integration

- **BuildContext/Visualize**: Require synced graph data
- **SearchMemories**: Uses FTS index + concept graph
- **WriteMemory/EditMemory**: Generate content that Sync processes
- **SequentialThinking/Workflow**: Create sessions Sync monitors
- **DB File Watcher**: After incremental sync activity settles (2s debounce), triggers community recomputation automatically. Skips its own writes to avoid feedback loops.

## Troubleshooting

- **Database locked**: Wait for concurrent ops, retry
- **No concepts found**: Ensure files contain `[[WikiLink]]` references like [[research]], [[analysis]]
- **Slow performance**: Normal for large knowledge bases
- **Fast re-sync**: Normal — mtime optimization skips unchanged files without reading them
- **Abandoned sessions warning**: Expected cleanup behavior
