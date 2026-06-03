import { describe, expect, it } from 'vitest';
import {
  buildHistoryRows,
  computeDuration,
  formatDuration,
  sortHistory,
  toEpochMs,
  type HistoryRow,
} from './runnerHistory';
import type { PipelineListItem, PipelineView, ViewNode } from '../../../lib/pipeline-types';

function node(partial: Partial<ViewNode> & { nodeId: string }): ViewNode {
  const base = {
    nodeId: partial.nodeId,
    sessionId: `s-${partial.nodeId}`,
    status: 'completed',
    attempts: 0,
    type: 'prompt',
    prompt: 'p',
    agent: 'copilot',
  };
  return { ...base, ...partial } as ViewNode;
}

function view(nodes: ViewNode[]): PipelineView {
  return {
    pipelineId: 'pid',
    name: 'demo',
    status: 'active',
    nodes,
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: false,
  };
}

describe('runnerHistory.toEpochMs', () => {
  it('passes finite numbers through', () => {
    expect(toEpochMs(1000)).toBe(1000);
    expect(toEpochMs(0)).toBe(0);
  });

  it('parses ISO-8601 strings', () => {
    expect(toEpochMs('2026-06-03T04:16:15.408Z')).toBe(Date.parse('2026-06-03T04:16:15.408Z'));
  });

  it('returns null for null/undefined/unparseable/non-finite', () => {
    expect(toEpochMs(null)).toBeNull();
    expect(toEpochMs(undefined)).toBeNull();
    expect(toEpochMs('not a date')).toBeNull();
    expect(toEpochMs(Number.POSITIVE_INFINITY)).toBeNull();
  });
});

describe('runnerHistory.computeDuration', () => {
  it('computes a non-negative delta from mixed number/string inputs', () => {
    expect(computeDuration(1000, 3500)).toBe(2500);
    expect(computeDuration('2026-06-03T00:00:00.000Z', '2026-06-03T00:00:01.000Z')).toBe(1000);
  });

  it('returns null when either bound is missing or the delta is negative', () => {
    expect(computeDuration(null, 1000)).toBeNull();
    expect(computeDuration(1000, null)).toBeNull();
    expect(computeDuration(5000, 1000)).toBeNull();
  });
});

describe('runnerHistory.formatDuration', () => {
  it('formats millis, seconds and minutes; em-dash for null', () => {
    expect(formatDuration(null)).toBe('—');
    expect(formatDuration(450)).toBe('450 ms');
    expect(formatDuration(2500)).toBe('2.50 s');
    expect(formatDuration(15000)).toBe('15.0 s');
    expect(formatDuration(90000)).toBe('1m 30s');
  });
});

describe('runnerHistory.buildHistoryRows', () => {
  it('builds one row per node and merges outcome from the list item runs', () => {
    const v = view([
      node({ nodeId: 'n1', displayLabel: 'Plan', status: 'completed', startedAt: 1000, completedAt: 1500 }),
      node({ nodeId: 'n2', status: 'pending' }),
    ]);
    const listItem = {
      pipelineId: 'pid',
      status: 'active',
      nodeCount: 2,
      completedCount: 1,
      runs: [{ nodeId: 'n1', startedAt: 1000, completedAt: 1500, status: 'completed', outcome: 'succeeded' }],
    } as PipelineListItem;

    const rows = buildHistoryRows(v, listItem);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({ nodeId: 'n1', label: 'Plan', outcome: 'succeeded', durationMs: 500 });
    expect(rows[1]).toMatchObject({ nodeId: 'n2', label: 'n2', outcome: null, durationMs: null });
  });

  it('tolerates a missing list item', () => {
    const rows = buildHistoryRows(view([node({ nodeId: 'n1' })]));
    expect(rows[0].outcome).toBeNull();
  });
});

describe('runnerHistory.sortHistory', () => {
  const rows: HistoryRow[] = [
    { nodeId: 'b', label: 'Bravo', status: 'completed', outcome: null, startedAt: 100, completedAt: 200, durationMs: 100 },
    { nodeId: 'a', label: 'Alpha', status: 'completed', outcome: null, startedAt: 300, completedAt: 400, durationMs: 100 },
    { nodeId: 'c', label: 'Charlie', status: 'pending', outcome: null, startedAt: null, completedAt: null, durationMs: null },
  ];

  it('newest first by completion time, unstarted last', () => {
    const out = sortHistory(rows, 'newest').map((r) => r.nodeId);
    expect(out).toEqual(['a', 'b', 'c']);
  });

  it('oldest first by completion time, unstarted last', () => {
    const out = sortHistory(rows, 'oldest').map((r) => r.nodeId);
    expect(out).toEqual(['b', 'a', 'c']);
  });

  it('by node name alphabetically', () => {
    const out = sortHistory(rows, 'node').map((r) => r.label);
    expect(out).toEqual(['Alpha', 'Bravo', 'Charlie']);
  });

  it('does not mutate the input array', () => {
    const before = rows.map((r) => r.nodeId);
    sortHistory(rows, 'newest');
    expect(rows.map((r) => r.nodeId)).toEqual(before);
  });
});
