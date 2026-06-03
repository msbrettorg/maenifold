import { describe, expect, it, vi } from 'vitest';
import { preflightJoin } from './joinPreflight';
import type { NodeStatus, PipelineView, ViewNode } from './pipeline-types';

// preflightJoin is the submit-time guard extracted from JoinPipelineDialog.vue
// (T-600.5 Defect 2). Its job is to RE-FETCH the source live and re-check both
// pipelines' status, so a source that began running AFTER it was selected still
// blocks the join. These tests pin that behavior.

function node(status: NodeStatus, nodeId = 'n1'): ViewNode {
  return { type: 'workflow', nodeId, sessionId: `${nodeId}-sess`, status, attempts: 0, workflowId: 'wf' };
}

function view(pipelineId: string, nodes: ViewNode[], running = false): PipelineView {
  return {
    pipelineId,
    status: 'active',
    nodes,
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running,
  };
}

const idleTarget = view('target', [node('completed', 't1')]);

describe('preflightJoin', () => {
  it('rejects an empty source', async () => {
    const fetchSource = vi.fn();
    const res = await preflightJoin({
      sourceId: '',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [],
      fetchSource,
    });
    expect(res.ok).toBe(false);
    expect(fetchSource).not.toHaveBeenCalled();
  });

  it('rejects source === target', async () => {
    const res = await preflightJoin({
      sourceId: 'target',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [],
      fetchSource: vi.fn(),
    });
    expect(res).toEqual({ ok: false, error: 'Source and target must differ.' });
  });

  it('blocks when the live re-fetch shows the source is now running', async () => {
    // The cached snapshot was idle; the LIVE fetch returns an active node. The
    // guard must re-fetch and block on the FRESH state, not the snapshot.
    const fetchSource = vi.fn().mockResolvedValue(view('source', [node('active', 's1')]));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [],
      fetchSource,
    });
    expect(fetchSource).toHaveBeenCalledWith('source');
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error).toMatch(/running/i);
  });

  it('blocks when the live target is running', async () => {
    const fetchSource = vi.fn().mockResolvedValue(view('source', [node('completed', 's1')]));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: view('target', [node('active', 't1')]),
      targetNodeIds: ['t1'],
      edgeDrafts: [],
      fetchSource,
    });
    expect(res.ok).toBe(false);
    // Target is checked before the source fetch.
    expect(fetchSource).not.toHaveBeenCalled();
  });

  it('flags node-id collisions using the FRESH source nodes', async () => {
    const fetchSource = vi.fn().mockResolvedValue(view('source', [node('completed', 'shared')]));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['shared'],
      edgeDrafts: [],
      fetchSource,
    });
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error).toMatch(/collision/i);
  });

  it('rejects a late/stale fetch whose id no longer matches the selection', async () => {
    const fetchSource = vi.fn().mockResolvedValue(view('other', [node('completed', 'x1')]));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [],
      fetchSource,
    });
    expect(res).toEqual({ ok: false, error: 'Source selection changed; please retry.' });
  });

  it('surfaces a thrown fetch error', async () => {
    const fetchSource = vi.fn().mockRejectedValue(new Error('network down'));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [],
      fetchSource,
    });
    expect(res).toEqual({ ok: false, error: 'network down' });
  });

  it('passes and builds edges when both pipelines are idle and ids are distinct', async () => {
    const fetchSource = vi.fn().mockResolvedValue(view('source', [node('completed', 's1')]));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [{ source: 't1', target: 's1', type: 'success' }],
      fetchSource,
    });
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.sourceView.pipelineId).toBe('source');
      expect(res.edges).toEqual([{ source: 't1', target: 's1', type: 'success' }]);
    }
  });

  it('blocks an edge whose endpoint is absent from the re-fetched fresh source', async () => {
    // The draft was authored against a source node ('ghost') that no longer
    // exists in the LIVE re-fetch. Status/collision checks pass, but the
    // endpoint validation must block before the daemon would reject it.
    const fetchSource = vi.fn().mockResolvedValue(view('source', [node('completed', 's1')]));
    const res = await preflightJoin({
      sourceId: 'source',
      targetId: 'target',
      targetView: idleTarget,
      targetNodeIds: ['t1'],
      edgeDrafts: [{ source: 't1', target: 'ghost', type: 'success' }],
      fetchSource,
    });
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.error).toMatch(/ghost/);
      expect(res.error).toMatch(/no longer exists/i);
    }
  });
});
