import { describe, expect, it } from 'vitest';
import { diffViews, isTerminalStatus, type LogEntry } from './runnerLog';
import type { PipelineStatus, PipelineView, ViewNode } from '../../../lib/pipeline-types';

// Minimal ViewNode factory — only the fields diffViews reads.
function node(partial: Partial<ViewNode> & { nodeId: string }): ViewNode {
  const base = {
    nodeId: partial.nodeId,
    sessionId: `s-${partial.nodeId}`,
    status: 'pending',
    attempts: 0,
    type: 'prompt',
    prompt: 'p',
    agent: 'copilot',
  };
  return { ...base, ...partial } as ViewNode;
}

function view(partial: Partial<PipelineView> & { nodes: ViewNode[] }): PipelineView {
  const base: PipelineView = {
    pipelineId: 'pid',
    name: 'demo',
    status: 'active',
    nodes: [],
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: false,
  };
  return { ...base, ...partial };
}

const NOW = 1_700_000_000_000;

describe('runnerLog.isTerminalStatus', () => {
  it('treats completed/abandoned/cancelled/stuck/failed as terminal', () => {
    for (const s of ['completed', 'abandoned', 'cancelled', 'stuck', 'failed'] as PipelineStatus[]) {
      expect(isTerminalStatus(s)).toBe(true);
    }
  });

  it('treats running/active/paused/pending/retrying as non-terminal', () => {
    for (const s of ['running', 'active', 'paused', 'pending', 'retrying'] as PipelineStatus[]) {
      expect(isTerminalStatus(s)).toBe(false);
    }
  });

  it('returns false for null/undefined', () => {
    expect(isTerminalStatus(null)).toBe(false);
    expect(isTerminalStatus(undefined)).toBe(false);
  });
});

describe('runnerLog.diffViews', () => {
  it('emits a single watching entry on the first tick (prev === null) and no per-node spam', () => {
    const next = view({ status: 'active', nodes: [node({ nodeId: 'n1' }), node({ nodeId: 'n2' })] });
    const entries = diffViews(null, next, NOW);
    expect(entries).toHaveLength(1);
    expect(entries[0].nodeId).toBeNull();
    expect(entries[0].message).toContain('Watching');
    expect(entries[0].message).toContain('active');
    expect(entries[0].id).toBe(`${NOW}-0`);
  });

  it('logs a pipeline status transition', () => {
    const prev = view({ status: 'active', nodes: [node({ nodeId: 'n1' })] });
    const next = view({ status: 'completed', nodes: [node({ nodeId: 'n1' })] });
    const entries = diffViews(prev, next, NOW);
    const pipelineEntry = entries.find((e) => e.nodeId === null);
    expect(pipelineEntry?.message).toBe('Pipeline status: active → completed.');
    expect(pipelineEntry?.level).toBe('success');
  });

  it('logs node status transitions with the mapped level', () => {
    const prev = view({ nodes: [node({ nodeId: 'n1', status: 'active', displayLabel: 'Plan' })] });
    const next = view({ nodes: [node({ nodeId: 'n1', status: 'failed', displayLabel: 'Plan' })] });
    const entries = diffViews(prev, next, NOW);
    const nodeEntry = entries.find((e) => e.nodeId === 'n1');
    expect(nodeEntry?.message).toBe('Plan: active → failed.');
    expect(nodeEntry?.level).toBe('error');
  });

  it('logs first-output appearance (empty -> non-empty) but not on subsequent ticks', () => {
    const prev = view({ nodes: [node({ nodeId: 'n1', output: null })] });
    const mid = view({ nodes: [node({ nodeId: 'n1', output: 'hello world' })] });
    const firstOut = diffViews(prev, mid, NOW).filter((e) => e.message.includes('produced output'));
    expect(firstOut).toHaveLength(1);
    expect(firstOut[0].message).toContain('11 chars');

    // Output unchanged on the next tick => no new output entry.
    const next = view({ nodes: [node({ nodeId: 'n1', output: 'hello world' })] });
    const secondOut = diffViews(mid, next, NOW).filter((e) => e.message.includes('produced output'));
    expect(secondOut).toHaveLength(0);
  });

  it('logs a grafted node only when prev exists', () => {
    const prev = view({ nodes: [node({ nodeId: 'n1' })] });
    const next = view({ nodes: [node({ nodeId: 'n1' }), node({ nodeId: 'n2', displayLabel: 'Added' })] });
    const entries = diffViews(prev, next, NOW);
    const added = entries.find((e) => e.message.startsWith('Node added'));
    expect(added?.message).toBe('Node added: Added.');
    expect(added?.nodeId).toBe('n2');
  });

  it('produces unique ids within a tick', () => {
    const prev = view({
      status: 'active',
      nodes: [node({ nodeId: 'n1', status: 'pending' }), node({ nodeId: 'n2', status: 'pending' })],
    });
    const next = view({
      status: 'completed',
      nodes: [node({ nodeId: 'n1', status: 'completed' }), node({ nodeId: 'n2', status: 'active' })],
    });
    const entries = diffViews(prev, next, NOW);
    const ids = new Set(entries.map((e: LogEntry) => e.id));
    expect(ids.size).toBe(entries.length);
    expect(entries.length).toBeGreaterThanOrEqual(3);
  });

  it('emits no entries when nothing changed', () => {
    const a = view({ nodes: [node({ nodeId: 'n1', status: 'active', output: 'x' })] });
    const b = view({ nodes: [node({ nodeId: 'n1', status: 'active', output: 'x' })] });
    expect(diffViews(a, b, NOW)).toHaveLength(0);
  });
});
