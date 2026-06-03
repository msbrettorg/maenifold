// Submit-time preflight for the Join dialog (T-600.5 Defect 2). Extracted from
// JoinPipelineDialog.vue so the re-fetch-and-block decision is unit-testable in
// the node env (the SFC body can't be mounted there).
//
// The critical invariant: the SOURCE pipeline status is re-fetched LIVE at submit
// time (not trusted from the snapshot taken at selection), because the daemon
// rejects a join when EITHER source or target has Ready|Active|Retrying nodes
// (handler_surgery_join.rs:133-150). A source that started running after it was
// picked must block the join, and edge wiring/collision checks must use the
// FRESH node list, never the stale cache.

import { canJoin } from './surgeryEnablement';
import { buildEdges, collisions, type EdgeDraft } from './surgeryEdges';
import type { PipelineEdge, PipelineView } from './pipeline-types';

export type JoinPreflightResult =
  | { ok: true; sourceView: PipelineView; edges: PipelineEdge[] }
  | { ok: false; error: string };

export interface JoinPreflightInput {
  sourceId: string;
  targetId: string | null | undefined;
  /** Live target view (reactive selection) — re-checked here. */
  targetView: PipelineView | null;
  targetNodeIds: readonly string[];
  edgeDrafts: readonly EdgeDraft[];
  /** Authoritative, non-mutating live fetch of the source view (store.fetchViewRaw). */
  fetchSource: (id: string) => Promise<PipelineView>;
}

/**
 * Validate a join at submit time. Re-fetches the source live, re-checks both
 * pipelines' status, preflights id collisions against the FRESH source nodes,
 * and builds the wire edges. Pure aside from the injected `fetchSource`.
 */
export async function preflightJoin(input: JoinPreflightInput): Promise<JoinPreflightResult> {
  const { sourceId, targetId, targetView, targetNodeIds, edgeDrafts, fetchSource } = input;

  if (!sourceId) return { ok: false, error: 'Pick a source pipeline to join.' };
  if (sourceId === targetId) return { ok: false, error: 'Source and target must differ.' };

  const target = canJoin(targetView);
  if (!target.enabled) {
    return { ok: false, error: target.reason ?? 'Target pipeline is running; cannot join.' };
  }

  let freshSource: PipelineView;
  try {
    freshSource = await fetchSource(sourceId);
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
  if (freshSource.pipelineId !== sourceId) {
    return { ok: false, error: 'Source selection changed; please retry.' };
  }

  const source = canJoin(freshSource);
  if (!source.enabled) {
    return { ok: false, error: source.reason ?? 'Source pipeline is running; cannot join.' };
  }

  const sourceIds = freshSource.nodes.map((node) => node.nodeId);
  const clash = collisions(targetNodeIds, sourceIds);
  if (clash.length > 0) {
    return { ok: false, error: `Node id collision between pipelines: ${clash.join(', ')}` };
  }

  const built = buildEdges(edgeDrafts);
  if ('error' in built) {
    return { ok: false, error: `Edge ${built.index + 1}: ${built.error}` };
  }

  // Validate every built edge endpoint against the FRESH merged node set
  // (target nodes + re-fetched source nodes). Stale drafts may reference source
  // nodes that were removed/renamed since selection; the daemon's
  // validate_pipeline rejects unknown endpoints (validation.rs:102-105,153-174),
  // so block here with a precise message rather than round-tripping a failure.
  const validIds = new Set<string>([
    ...targetNodeIds,
    ...freshSource.nodes.map((node) => node.nodeId),
  ]);
  for (let index = 0; index < built.edges.length; index += 1) {
    const edge = built.edges[index];
    if (!validIds.has(edge.source)) {
      return {
        ok: false,
        error: `Edge ${index + 1}: source node "${edge.source}" no longer exists in the source/target pipeline.`,
      };
    }
    if (!validIds.has(edge.target)) {
      return {
        ok: false,
        error: `Edge ${index + 1}: target node "${edge.target}" no longer exists in the source/target pipeline.`,
      };
    }
  }

  return { ok: true, sourceView: freshSource, edges: built.edges };
}
