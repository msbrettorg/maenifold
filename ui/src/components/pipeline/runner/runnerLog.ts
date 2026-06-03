// Pipeline runner — event-log derivation (pure, DOM-free, unit-testable).
//
// `diffViews` turns two successive `PipelineView` snapshots (the previous poll
// and the current poll) into a list of `LogEntry` rows describing what changed:
// pipeline-level status transitions, per-node status transitions, and first
// appearance of node output. The live runner (usePipelineRunner.ts) calls this
// every poll tick and appends the result to the store's event log. Keeping the
// diff here — separate from any interval/DOM concern — is what makes it testable.

import type {
  NodeStatus,
  PipelineStatus,
  PipelineView,
  ViewNode,
} from '../../../lib/pipeline-types';

export type LogLevel = 'info' | 'success' | 'warn' | 'error';

export interface LogEntry {
  // Stable-per-tick id: `${pollEpochMs}-${sequence}`. Unique within a poll and
  // monotonic across polls, so it is a safe Vue `:key`.
  id: string;
  // Epoch milliseconds the entry was derived (the poll tick's `now`).
  at: number;
  level: LogLevel;
  // The node the entry concerns, or null for pipeline-level entries.
  nodeId: string | null;
  message: string;
}

// Terminal pipeline statuses: once the pipeline reaches one of these it can no
// longer make progress, so polling must stop. Mirrors the backend terminal set
// (completed / abandoned / cancelled / stuck / failed). `cancelled` is the
// backend serde alias for `abandoned` but can surface at the pipeline level too.
const TERMINAL_STATUSES: ReadonlySet<PipelineStatus> = new Set<PipelineStatus>([
  'completed',
  'abandoned',
  'cancelled',
  'stuck',
  'failed',
]);

export function isTerminalStatus(status: PipelineStatus | null | undefined): boolean {
  return status != null && TERMINAL_STATUSES.has(status);
}

function nodeLevel(status: NodeStatus): LogLevel {
  switch (status) {
    case 'completed':
      return 'success';
    case 'failed':
    case 'stuck':
      return 'error';
    case 'abandoned':
    case 'cancelled':
    case 'skipped':
      return 'warn';
    default:
      return 'info';
  }
}

function pipelineLevel(status: PipelineStatus): LogLevel {
  if (status === 'completed') return 'success';
  if (status === 'failed' || status === 'stuck') return 'error';
  if (status === 'abandoned' || status === 'cancelled' || status === 'paused') return 'warn';
  return 'info';
}

function nodeLabel(node: ViewNode): string {
  return node.displayLabel ?? node.nodeId;
}

function outputText(node: ViewNode): string {
  return (node.output ?? '').trim();
}

// Diff two successive pipeline views into log entries. `prev` is null on the
// first tick of a watch session (the runner resets it on pipeline switch /
// run), which yields a single "watching" entry and suppresses per-node spam for
// the initial snapshot.
export function diffViews(
  prev: PipelineView | null,
  next: PipelineView,
  now: number
): LogEntry[] {
  const entries: LogEntry[] = [];
  let seq = 0;
  const push = (level: LogLevel, nodeId: string | null, message: string): void => {
    entries.push({ id: `${now}-${seq}`, at: now, level, nodeId, message });
    seq += 1;
  };

  if (!prev) {
    push('info', null, `Watching ${next.name ?? next.pipelineId} (status: ${next.status}).`);
  } else if (prev.status !== next.status) {
    push(pipelineLevel(next.status), null, `Pipeline status: ${prev.status} → ${next.status}.`);
  }

  const prevById = new Map<string, ViewNode>();
  for (const node of prev?.nodes ?? []) {
    prevById.set(node.nodeId, node);
  }

  for (const node of next.nodes) {
    const before = prevById.get(node.nodeId);
    const label = nodeLabel(node);

    if (!before) {
      // A node that appears only in `next` is a graft. On the very first tick
      // (prev === null) every node is "new"; suppress those to avoid logging the
      // entire graph — the "watching" line already covers the initial state.
      if (prev) push('info', node.nodeId, `Node added: ${label}.`);
      continue;
    }

    if (before.status !== node.status) {
      push(nodeLevel(node.status), node.nodeId, `${label}: ${before.status} → ${node.status}.`);
    }

    const beforeOut = outputText(before);
    const afterOut = outputText(node);
    if (!beforeOut && afterOut) {
      push('info', node.nodeId, `${label} produced output (${afterOut.length} chars).`);
    }
  }

  return entries;
}
