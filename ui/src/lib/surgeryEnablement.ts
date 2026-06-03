// Surgery enablement matrix (T-600.5). Pure predicates that decide, for the
// SELECTED LIVE pipeline, which graph-surgery operation is currently legal. The
// daemon enforces each precondition and returns an error if violated; the UI
// disables (and tooltips) the illegal action instead of optimistically issuing
// a call that will fail. Each predicate mirrors a verified backend guard:
//
//  - rewind  : crates/maenifold-core/src/workflow/pipeline/handler_lifecycle_rewind.rs:47-51
//              (rejects a held/running lease) — cascade resets at :57-65.
//  - restart : update_restart.rs:23-27 (rejects running lease),
//              :52-60 (only Abandoned|Stuck|Failed may restart).
//  - graft   : handler_surgery_graft.rs:38-43 (rejects remaining pending/active
//              work via has_remaining_work) — collisions at :125-132.
//  - join    : handler_surgery_join.rs:133-150 (rejects Ready|Active|Retrying in
//              source OR target) — collisions at :54-64.
//  - abandon : update_status.rs:144-149 (node must currently be Active).
//  - fork    : handler_surgery_fork.rs has no status guard; only a duplicate
//              new-session-id (:39-44) or missing source node (:51-59) fail.
//
// The "remaining work" set mirrors engine_ready.rs:89-96
// (Pending | Ready | Active | Retrying).

import type { NodeStatus, PipelineView, ViewNode } from './pipeline-types';

export interface Enablement {
  enabled: boolean;
  // Present only when disabled: the tooltip explaining why.
  reason?: string;
}

// Restartable node statuses. The backend matches Abandoned | Stuck | Failed;
// `cancelled` is the serde alias the view may emit for `abandoned`, so it is
// accepted here as the same restartable state.
const RESTARTABLE: ReadonlySet<NodeStatus> = new Set<NodeStatus>([
  'abandoned',
  'cancelled',
  'stuck',
  'failed',
]);

// `has_remaining_work` (graft guard): Pending | Ready | Active | Retrying.
const REMAINING_WORK: ReadonlySet<NodeStatus> = new Set<NodeStatus>([
  'pending',
  'ready',
  'active',
  'retrying',
]);

// Join's blocking set: Ready | Active | Retrying (NOT pending).
const JOIN_BLOCKING: ReadonlySet<NodeStatus> = new Set<NodeStatus>([
  'ready',
  'active',
  'retrying',
]);

const ENABLED: Enablement = { enabled: true };

function disabled(reason: string): Enablement {
  return { enabled: false, reason };
}

/**
 * Lease proxy: the view's `running` flag is true while a pipeline-driver holds
 * the session lock and a node is executing. rewind/restart reject a held lease.
 */
export function leaseHeld(view: PipelineView | null): boolean {
  return view?.running === true;
}

/** Any node in Pending | Ready | Active | Retrying (graft's `has_remaining_work`). */
export function hasRemainingWork(view: PipelineView | null): boolean {
  return Boolean(view?.nodes.some((node) => REMAINING_WORK.has(node.status)));
}

/** Any node in Ready | Active | Retrying (join's blocking set). */
export function hasJoinBlockingWork(view: PipelineView | null): boolean {
  return Boolean(view?.nodes.some((node) => JOIN_BLOCKING.has(node.status)));
}

export function isRestartableStatus(status: NodeStatus): boolean {
  return RESTARTABLE.has(status);
}

// --- per-operation predicates ----------------------------------------------

/** Single-node rewind: legal only when no lease is held. */
export function canRewindNode(view: PipelineView | null): Enablement {
  if (!view) return disabled('No pipeline selected');
  if (leaseHeld(view)) {
    return disabled('Pipeline is running; cancel it before rewinding');
  }
  return ENABLED;
}

/** Batch rewind: ≥1 selected id and no lease held. Issues ONE call for all ids. */
export function canBatchRewind(
  view: PipelineView | null,
  nodeIds: readonly string[]
): Enablement {
  if (nodeIds.length === 0) return disabled('Select one or more nodes to rewind');
  return canRewindNode(view);
}

/** Restart: no lease held AND the node is Abandoned | Stuck | Failed. */
export function canRestartNode(
  view: PipelineView | null,
  node: ViewNode | null
): Enablement {
  if (!view || !node) return disabled('No node selected');
  if (leaseHeld(view)) {
    return disabled('Pipeline is running; cancel it before restarting a node');
  }
  if (!isRestartableStatus(node.status)) {
    return disabled('Only abandoned, stuck, or failed nodes can be restarted');
  }
  return ENABLED;
}

/** Abandon: only an Active node may transition to abandoned. */
export function canAbandonNode(node: ViewNode | null): Enablement {
  if (!node) return disabled('No node selected');
  if (node.status !== 'active') {
    return disabled('Only an active node can be abandoned');
  }
  return ENABLED;
}

/** Graft: rejected while any node still has remaining work. */
export function canGraft(view: PipelineView | null): Enablement {
  if (!view) return disabled('No pipeline selected');
  if (hasRemainingWork(view)) {
    return disabled('Pipeline has pending or active nodes; complete or cancel before grafting');
  }
  return ENABLED;
}

/** Join: target rejected while any node is Ready | Active | Retrying. */
export function canJoin(view: PipelineView | null): Enablement {
  if (!view) return disabled('No pipeline selected');
  if (hasJoinBlockingWork(view)) {
    return disabled('Pipeline is running; complete or cancel before joining');
  }
  return ENABLED;
}

/** Fork: legal for any existing node (no backend status guard). */
export function canForkNode(node: ViewNode | null): Enablement {
  if (!node) return disabled('No node selected');
  return ENABLED;
}
