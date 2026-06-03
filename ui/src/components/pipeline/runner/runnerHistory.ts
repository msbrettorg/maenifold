// Pipeline runner — run-history derivation (pure, DOM-free, unit-testable).
//
// Builds the run-history table from a `PipelineView` (the per-node runtime
// state) merged with `PipelineListItem.runs` (which carries the per-run
// `outcome`). Computes per-node duration from startedAt/completedAt (the backend
// emits either epoch-ms numbers or ISO-8601 strings, so `toEpochMs` normalizes
// both), formats it for display, and sorts rows newest / oldest / by node name.

import type { PipelineListItem, PipelineView } from '../../../lib/pipeline-types';

export type HistorySort = 'newest' | 'oldest' | 'node';

export interface HistoryRow {
  nodeId: string;
  label: string;
  status: string;
  outcome: string | null;
  startedAt: number | null;
  completedAt: number | null;
  durationMs: number | null;
}

// Normalize a timestamp to epoch milliseconds. The view emits epoch-ms numbers
// for node start/complete, but the list `runs` and some node fields can be ISO
// strings; `Date.parse` handles the ISO case. Returns null for missing/unparseable.
export function toEpochMs(value: string | number | null | undefined): number | null {
  if (value == null) return null;
  if (typeof value === 'number') return Number.isFinite(value) ? value : null;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? null : parsed;
}

export function computeDuration(
  startedAt: string | number | null | undefined,
  completedAt: string | number | null | undefined
): number | null {
  const start = toEpochMs(startedAt);
  const end = toEpochMs(completedAt);
  if (start == null || end == null) return null;
  const delta = end - start;
  return delta >= 0 ? delta : null;
}

export function formatDuration(ms: number | null): string {
  if (ms == null) return '—';
  if (ms < 1000) return `${ms} ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 2 : 1)} s`;
  const minutes = Math.floor(seconds / 60);
  const remainder = Math.round(seconds % 60);
  return `${minutes}m ${remainder}s`;
}

// Build one history row per view node, merging the per-run `outcome` from the
// list item (matched by nodeId) when available.
export function buildHistoryRows(
  view: PipelineView,
  listItem?: PipelineListItem | null
): HistoryRow[] {
  const outcomeByNode = new Map<string, string | null>();
  for (const run of listItem?.runs ?? []) {
    outcomeByNode.set(run.nodeId, run.outcome);
  }

  return view.nodes.map((node) => ({
    nodeId: node.nodeId,
    label: node.displayLabel ?? node.nodeId,
    status: node.status,
    outcome: outcomeByNode.get(node.nodeId) ?? null,
    startedAt: toEpochMs(node.startedAt),
    completedAt: toEpochMs(node.completedAt),
    durationMs: computeDuration(node.startedAt, node.completedAt),
  }));
}

function sortKey(row: HistoryRow): number | null {
  return row.completedAt ?? row.startedAt ?? null;
}

// Sort a copy of the rows. `newest`/`oldest` sort by completion (falling back to
// start) time with unstarted rows last; `node` sorts alphabetically by label.
export function sortHistory(rows: HistoryRow[], sort: HistorySort): HistoryRow[] {
  const copy = [...rows];
  if (sort === 'node') {
    copy.sort((a, b) => a.label.localeCompare(b.label));
    return copy;
  }
  copy.sort((a, b) => {
    const ka = sortKey(a);
    const kb = sortKey(b);
    if (ka == null && kb == null) return 0;
    if (ka == null) return 1; // unstarted rows sink to the bottom
    if (kb == null) return -1;
    return sort === 'newest' ? kb - ka : ka - kb;
  });
  return copy;
}
