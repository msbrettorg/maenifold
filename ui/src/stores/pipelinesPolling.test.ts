import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

// Blue-team coverage (T-600.3): the store's live-runner polling methods carry
// the two safety-critical behaviors the red-team fix depends on — `fetchViewRaw`
// MUST be non-mutating (so an in-flight poll for pipeline A can never clobber the
// selection after a switch to B) and `appendLogEntries` MUST cap the buffer at
// EVENT_LOG_CAP=500 (dropping oldest). The existing pipelines.test.ts covers the
// lifecycle → gateway op mapping but not these polling-state methods; this file
// closes that gap. Test-only; no production code is touched.
vi.mock('../lib/mcp/gateway', () => ({
  executeGateway: vi.fn(),
  readGatewayResource: vi.fn(),
}));

import { executeGateway } from '../lib/mcp/gateway';
import { usePipelinesStore } from './pipelines';
import type { LogEntry } from '../components/pipeline/runner/runnerLog';
import type { PipelineView } from '../lib/pipeline-types';

const executeGatewayMock = vi.mocked(executeGateway);

const WORKFLOW_DOMAIN = 'maenifold.workflow';
const PIPELINE_COMMAND = 'Pipeline';

function rawView(pipelineId: string, status = 'active'): Record<string, unknown> {
  return { pipelineId, status, nodes: [], edges: [] };
}

function entry(seq: number): LogEntry {
  return { id: `e-${seq}`, at: seq, level: 'info', nodeId: null, message: `m-${seq}` };
}

function liveView(pipelineId: string): PipelineView {
  return {
    pipelineId,
    name: pipelineId,
    status: 'active',
    nodes: [],
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: false,
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
  executeGatewayMock.mockReset();
  executeGatewayMock.mockResolvedValue(undefined as unknown as never);
});

describe('usePipelinesStore.fetchViewRaw — non-mutating poll read', () => {
  it('sends {sessionId, view:true} and returns a normalized view', async () => {
    executeGatewayMock.mockResolvedValueOnce(rawView('p1') as never);
    const store = usePipelinesStore();
    const view = await store.fetchViewRaw('sess-1');

    expect(executeGatewayMock).toHaveBeenCalledTimes(1);
    const [domain, command, params] = executeGatewayMock.mock.calls[0];
    expect(domain).toBe(WORKFLOW_DOMAIN);
    expect(command).toBe(PIPELINE_COMMAND);
    expect(params).toEqual({ sessionId: 'sess-1', view: true });
    expect(view.pipelineId).toBe('p1');
  });

  it('does NOT assign selectedView/selectedPipelineId (the no-stale-clobber guarantee)', async () => {
    const store = usePipelinesStore();
    // Seed a current selection (pipeline B), as if the user already switched.
    store.applyPolledView(liveView('B'));
    store.selectedPipelineId = 'B';

    // An in-flight poll for pipeline A resolves.
    executeGatewayMock.mockResolvedValueOnce(rawView('A') as never);
    await store.fetchViewRaw('A');

    // The selection must still be B — fetchViewRaw never touches it.
    expect(store.selectedPipelineId).toBe('B');
    expect(store.selectedView?.pipelineId).toBe('B');
  });

  it('does not clear or set the error ref on its own', async () => {
    const store = usePipelinesStore();
    store.error = 'prior error';
    executeGatewayMock.mockResolvedValueOnce(rawView('p1') as never);
    await store.fetchViewRaw('p1');
    // viewPipeline clears error; fetchViewRaw is a quiet read and leaves it.
    expect(store.error).toBe('prior error');
  });
});

describe('usePipelinesStore.appendLogEntries — EVENT_LOG_CAP=500 trimming', () => {
  it('appends entries in order below the cap', () => {
    const store = usePipelinesStore();
    store.appendLogEntries([entry(1), entry(2)]);
    store.appendLogEntries([entry(3)]);
    expect(store.eventLog.map((e) => e.id)).toEqual(['e-1', 'e-2', 'e-3']);
  });

  it('is a no-op for an empty batch', () => {
    const store = usePipelinesStore();
    store.appendLogEntries([entry(1)]);
    store.appendLogEntries([]);
    expect(store.eventLog).toHaveLength(1);
  });

  it('caps the buffer at 500, dropping the oldest entries', () => {
    const store = usePipelinesStore();
    const batch = Array.from({ length: 600 }, (_, i) => entry(i));
    store.appendLogEntries(batch);
    expect(store.eventLog).toHaveLength(500);
    // Oldest 100 dropped: first kept is e-100, last is e-599 (newest).
    expect(store.eventLog[0].id).toBe('e-100');
    expect(store.eventLog[store.eventLog.length - 1].id).toBe('e-599');
  });

  it('caps across multiple appends, always keeping the newest 500', () => {
    const store = usePipelinesStore();
    for (let i = 0; i < 510; i += 1) store.appendLogEntries([entry(i)]);
    expect(store.eventLog).toHaveLength(500);
    expect(store.eventLog[0].id).toBe('e-10');
    expect(store.eventLog[499].id).toBe('e-509');
  });
});

describe('usePipelinesStore.resetEventLog / clearSelection / applyPolledView', () => {
  it('resetEventLog empties the log and clears pollError but keeps the selection', () => {
    const store = usePipelinesStore();
    store.applyPolledView(liveView('A'));
    store.selectedPipelineId = 'A';
    store.appendLogEntries([entry(1), entry(2)]);
    store.setPollError('boom');

    store.resetEventLog();

    expect(store.eventLog).toHaveLength(0);
    expect(store.pollError).toBe(null);
    // Selection survives a log reset (used on run/switch before re-polling).
    expect(store.selectedView?.pipelineId).toBe('A');
    expect(store.selectedPipelineId).toBe('A');
  });

  it('clearSelection wipes view + id + log + pollError (post-delete clean state)', () => {
    const store = usePipelinesStore();
    store.applyPolledView(liveView('A'));
    store.selectedPipelineId = 'A';
    store.appendLogEntries([entry(1)]);
    store.setPollError('boom');

    store.clearSelection();

    expect(store.selectedView).toBe(null);
    expect(store.selectedPipelineId).toBe(null);
    expect(store.eventLog).toHaveLength(0);
    expect(store.pollError).toBe(null);
  });

  it('applyPolledView updates selectedView only, not selectedPipelineId', () => {
    const store = usePipelinesStore();
    store.selectedPipelineId = 'A';
    store.applyPolledView(liveView('A'));
    expect(store.selectedView?.pipelineId).toBe('A');
    // applyPolledView intentionally leaves the id ref to the caller's gating.
    expect(store.selectedPipelineId).toBe('A');
  });
});

describe('usePipelinesStore.setPollingActive / setPollError', () => {
  it('toggles pollingActive', () => {
    const store = usePipelinesStore();
    expect(store.pollingActive).toBe(false);
    store.setPollingActive(true);
    expect(store.pollingActive).toBe(true);
    store.setPollingActive(false);
    expect(store.pollingActive).toBe(false);
  });

  it('sets and clears pollError', () => {
    const store = usePipelinesStore();
    store.setPollError('strike-3');
    expect(store.pollError).toBe('strike-3');
    store.setPollError(null);
    expect(store.pollError).toBe(null);
  });
});
