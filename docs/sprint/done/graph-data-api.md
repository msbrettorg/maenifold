# graph-data-api

**Requirement**: FR-32
**Test Cases**: TC-32.9, TC-32.10
**Component**: crates/maenifold-core/src/api/graph.rs (new)

## Acceptance Criteria

1. `GET /api/graph/nodes` returns JSON array of concept nodes: `{ "name": "...", "file_count": N, "community_id": N|null, "degree": N }`, where `file_count` is derived as `SELECT COUNT(DISTINCT source_file) FROM concept_mentions WHERE concept_name = ?`, `community_id` returns `null` when the node has no community assignment, and `degree` is computed as the count of edges in `concept_graph` where `concept_a = name OR concept_b = name`
2. `GET /api/graph/edges` returns JSON array of co-occurrence edges: `{ "source": "...", "target": "...", "weight": N }`, with column mapping `concept_a → source`, `concept_b → target`, and raw integer `co_occurrence_count → weight`
3. Both endpoints query the existing SQLite graph database (same DB used by Sync, BuildContext, Visualize tools)
4. Optional query param `?community=<id>` filters nodes to the specified community and filters edges to those where both endpoints belong to that community
5. Optional query param `?depth=<N>&center=<concept>` returns the ego-graph around a concept (same logic as BuildContext tool)
6. Responses include CORS headers (same permissive policy as existing MCP HTTP endpoints)
7. Empty graph returns `[]` (not an error)

## Verdicts

- TC-32.9 (GET /api/graph/nodes returns concept nodes): ADD — test does not exist yet
- TC-32.10 (GET /api/graph/edges returns co-occurrence edges): ADD — test does not exist yet
