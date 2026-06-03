import { describe, expect, it } from 'vitest';
import {
  canAbandonNode,
  canBatchRewind,
  canForkNode,
  canGraft,
  canJoin,
  canRestartNode,
  canRewindNode,
  hasJoinBlockingWork,
  hasRemainingWork,
  isRestartableStatus,
  leaseHeld,
} from './surgeryEnablement';
import type { NodeStatus, PipelineView, ViewNode } from './pipeline-types';

// Each predicate mirrors a verified backend guard (see surgeryEnablement.ts
// header for crate file:line). These tests pin the enablement MATRIX so a future
// edit can't silently loosen a guard the daemon still enforces.

function node(status: NodeStatus, nodeId = 'n1'): ViewNode {
  return {
    type: 'workflow',
    nodeId,
    sessionId: `${nodeId}-sess`,
    status,
    attempts: 0,
    workflowId: 'wf',
  };
}

function view(opts: { running?: boolean; nodes?: ViewNode[] } = {}): PipelineView {
  return {
    pipelineId: 'p1',
    status: 'active',
    nodes: opts.nodes ?? [],
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: opts.running ?? false,
  };
}

describe('leaseHeld / work predicates', () => {
  it('leaseHeld follows the view.running flag', () => {
    expect(leaseHeld(null)).toBe(false);
    expect(leaseHeld(view({ running: false }))).toBe(false);
    expect(leaseHeld(view({ running: true }))).toBe(true);
  });

  it('hasRemainingWork = Pending | Ready | Active | Retrying', () => {
    expect(hasRemainingWork(view({ nodes: [node('completed')] }))).toBe(false);
    for (const s of ['pending', 'ready', 'active', 'retrying'] as NodeStatus[]) {
      expect(hasRemainingWork(view({ nodes: [node(s)] }))).toBe(true);
    }
  });

  it('hasJoinBlockingWork excludes pending (Ready | Active | Retrying only)', () => {
    expect(hasJoinBlockingWork(view({ nodes: [node('pending')] }))).toBe(false);
    for (const s of ['ready', 'active', 'retrying'] as NodeStatus[]) {
      expect(hasJoinBlockingWork(view({ nodes: [node(s)] }))).toBe(true);
    }
  });

  it('isRestartableStatus = abandoned | cancelled | stuck | failed', () => {
    for (const s of ['abandoned', 'cancelled', 'stuck', 'failed'] as NodeStatus[]) {
      expect(isRestartableStatus(s)).toBe(true);
    }
    for (const s of ['pending', 'ready', 'active', 'completed'] as NodeStatus[]) {
      expect(isRestartableStatus(s)).toBe(false);
    }
  });
});

describe('canRewindNode / canBatchRewind', () => {
  it('rejects no pipeline and a held lease', () => {
    expect(canRewindNode(null).enabled).toBe(false);
    expect(canRewindNode(view({ running: true })).enabled).toBe(false);
    expect(canRewindNode(view({ running: false })).enabled).toBe(true);
  });

  it('batch rewind requires at least one selected id', () => {
    expect(canBatchRewind(view(), []).enabled).toBe(false);
    expect(canBatchRewind(view(), ['a']).enabled).toBe(true);
    // A held lease still blocks even with ids selected.
    expect(canBatchRewind(view({ running: true }), ['a', 'b']).enabled).toBe(false);
  });
});

describe('canRestartNode', () => {
  it('requires a restartable status and no held lease', () => {
    expect(canRestartNode(view(), null).enabled).toBe(false);
    expect(canRestartNode(view(), node('active')).enabled).toBe(false);
    expect(canRestartNode(view(), node('failed')).enabled).toBe(true);
    expect(canRestartNode(view(), node('abandoned')).enabled).toBe(true);
    expect(canRestartNode(view({ running: true }), node('failed')).enabled).toBe(false);
  });
});

describe('canAbandonNode', () => {
  it('only an active node may be abandoned', () => {
    expect(canAbandonNode(null).enabled).toBe(false);
    expect(canAbandonNode(node('active')).enabled).toBe(true);
    expect(canAbandonNode(node('pending')).enabled).toBe(false);
    expect(canAbandonNode(node('completed')).enabled).toBe(false);
  });
});

describe('canGraft', () => {
  it('rejected while any node still has remaining work', () => {
    expect(canGraft(null).enabled).toBe(false);
    expect(canGraft(view({ nodes: [node('completed')] })).enabled).toBe(true);
    expect(canGraft(view({ nodes: [node('pending')] })).enabled).toBe(false);
    expect(canGraft(view({ nodes: [node('active')] })).enabled).toBe(false);
  });
});

describe('canJoin', () => {
  it('rejected while any node is Ready | Active | Retrying (pending is OK)', () => {
    expect(canJoin(null).enabled).toBe(false);
    expect(canJoin(view({ nodes: [node('completed')] })).enabled).toBe(true);
    // pending does NOT block a join (differs from graft).
    expect(canJoin(view({ nodes: [node('pending')] })).enabled).toBe(true);
    expect(canJoin(view({ nodes: [node('active')] })).enabled).toBe(false);
  });
});

describe('canForkNode', () => {
  it('legal for any existing node (no status guard)', () => {
    expect(canForkNode(null).enabled).toBe(false);
    expect(canForkNode(node('active')).enabled).toBe(true);
    expect(canForkNode(node('completed')).enabled).toBe(true);
    expect(canForkNode(node('pending')).enabled).toBe(true);
  });
});
