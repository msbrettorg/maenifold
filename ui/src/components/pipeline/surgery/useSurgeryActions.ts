// Surgery action runner (T-600.5) — the SINGLE owner of the surgery refresh
// cycle. Mutating Pipeline ops return raw gateway results and do NOT update
// `selectedView`; the live runner's poll loop also pauses in design mode and can
// race a just-issued mutation. So every surgery op flows through here:
//
//   pause polling → call the mutation → viewPipeline(selectedId) (re-selects and
//   refreshes the LIVE view) → refreshList() → resume polling (the runner's
//   `start` self-guards terminal status + design mode).
//
// This guarantees the RELOAD-persisted acceptance: the UI reflects exactly what
// the daemon stored, not the mutation's return value, and no in-flight poll can
// clobber the post-mutation view.

import { usePipelinesStore } from '../../../stores/pipelines';
import { usePipelineDesignStore } from '../../../stores/pipelineDesign';
import { useSurgeryController } from './useSurgeryController';
import type { PipelineEdge, PipelineNode } from '../../../lib/pipeline-types';

export interface SurgeryResult {
  ok: boolean;
  error?: string;
  // True when the user cancelled at the discard-changes guard (no mutation ran).
  cancelled?: boolean;
}

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useSurgeryActions() {
  const store = usePipelinesStore();
  const design = usePipelineDesignStore();
  const controller = useSurgeryController();

  const busy = controller.busy;
  const lastError = controller.lastError;
  // Set after rewind (which cascades downstream resets) so the UI can surface a
  // "downstream nodes were also reset" hint. Cleared at the start of each op.
  const notice = controller.notice;

  function ok(): SurgeryResult {
    return { ok: true };
  }

  function fail(message: string): SurgeryResult {
    lastError.value = message;
    return { ok: false, error: message };
  }

  // Wrap a mutation in the pause→mutate→view→refresh→resume cycle.
  async function runSurgery(
    mutate: (sessionId: string) => Promise<unknown>,
    successNotice?: string
  ): Promise<SurgeryResult> {
    const sessionId = store.selectedPipelineId;
    if (!sessionId) return fail('No pipeline selected');

    // Cross-pipeline leak guard (T-600.5 Defect 3): if a surgery surface was
    // opened against one pipeline and the user switched to another before
    // submitting, the captured target no longer matches the live selection.
    // Refuse rather than mutate the wrong (now-selected) pipeline.
    const captured = controller.targetSessionId.value;
    if (captured && captured !== sessionId) {
      return fail(
        'Pipeline selection changed since this action was opened; re-open it on the current pipeline.'
      );
    }

    // Defensive design-mode guard. The surgery menu/toolbar are suppressed while
    // design mode is active, but if a dirty design session is somehow current we
    // route through the existing discard guard BEFORE any server mutation.
    if (design.designModeActive) {
      if (!design.confirmDiscardDesignChanges()) return { ok: false, cancelled: true };
      design.exitDesignMode();
    }

    busy.value = true;
    lastError.value = null;
    notice.value = null;
    controller.pausePolling();
    try {
      await mutate(sessionId);
      // Re-read persisted state into the live selection; never trust the
      // mutation's return value for UI state.
      await store.viewPipeline(sessionId);
      await store.refreshList();
      if (successNotice) notice.value = successNotice;
      return ok();
    } catch (error) {
      const message = toMessage(error);
      lastError.value = message;
      // Re-view so the UI shows the real (unchanged) server state after a
      // rejected op rather than a stale optimistic graph.
      try {
        await store.viewPipeline(sessionId);
      } catch {
        // Leave the prior view in place if the re-read also fails.
      }
      return { ok: false, error: message };
    } finally {
      // The runner's start guards terminal status + design mode, so this only
      // restarts polling when appropriate.
      controller.resumePolling();
      busy.value = false;
    }
  }

  // --- operations -----------------------------------------------------------

  /** Batch (or single) rewind — ONE rewindNodes call for ALL ids. */
  async function rewind(nodeIds: readonly string[]): Promise<SurgeryResult> {
    if (nodeIds.length === 0) return fail('No nodes to rewind');
    const ids = [...nodeIds];
    const hint =
      ids.length === 1
        ? 'Node rewound. Downstream nodes were also reset.'
        : `${ids.length} nodes rewound. Downstream nodes were also reset.`;
    return runSurgery((sessionId) => store.rewindNodes(sessionId, ids), hint);
  }

  /** Graft new nodes + linking edges into the live pipeline. */
  async function graft(
    nodes: PipelineNode[],
    edges: PipelineEdge[]
  ): Promise<SurgeryResult> {
    if (nodes.length === 0) return fail('Graft needs at least one node');
    return runSurgery((sessionId) => store.graftNodes(sessionId, nodes, edges));
  }

  /** Merge a source pipeline's nodes into the live pipeline with linking edges. */
  async function join(
    sourceSessionId: string,
    edges: PipelineEdge[]
  ): Promise<SurgeryResult> {
    if (!sourceSessionId.trim()) return fail('Join needs a source pipeline');
    return runSurgery((sessionId) => store.joinPipeline(sessionId, sourceSessionId, edges));
  }

  /** Fork a new pipeline from a node's reachable subgraph. */
  async function fork(
    fromNodeId: string,
    newSessionId?: string
  ): Promise<SurgeryResult> {
    if (!fromNodeId.trim()) return fail('Fork needs a source node');
    const target = newSessionId?.trim();
    return runSurgery((sessionId) =>
      store.forkPipeline(sessionId, fromNodeId, target ? target : undefined)
    );
  }

  /** Restart an Abandoned | Stuck | Failed node (sends {nodeId, restart:true}). */
  async function restartNode(nodeId: string): Promise<SurgeryResult> {
    if (!nodeId.trim()) return fail('No node to restart');
    return runSurgery((sessionId) => store.restartNode(sessionId, nodeId));
  }

  /** Abandon an Active node (optional conclusion). */
  async function abandonNode(nodeId: string, conclusion?: string): Promise<SurgeryResult> {
    if (!nodeId.trim()) return fail('No node to abandon');
    return runSurgery((sessionId) =>
      store.setNodeStatus(sessionId, nodeId, 'abandoned', conclusion)
    );
  }

  return {
    busy,
    lastError,
    notice,
    rewind,
    graft,
    join,
    fork,
    restartNode,
    abandonNode,
  };
}
