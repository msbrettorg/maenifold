import { beforeEach, describe, expect, it, vi } from 'vitest';

// The surgery refresh cycle is the safety-critical contract: every mutation MUST
// pause polling, mutate, RE-VIEW the persisted state (never trust the mutation's
// return value), refresh the list, and resume polling — in that exact order, and
// resume even on failure. Batch rewind MUST issue ONE rewindNodes call for ALL
// ids. We mock the two stores so the order is observable without a DOM or daemon.

const order: string[] = [];

const fakeStore = {
  selectedPipelineId: 'p1' as string | null,
  rewindNodes: vi.fn(async (_s: string, _ids: string[]) => {
    order.push('rewindNodes');
  }),
  graftNodes: vi.fn(async () => {
    order.push('graftNodes');
  }),
  joinPipeline: vi.fn(async () => {
    order.push('joinPipeline');
  }),
  forkPipeline: vi.fn(async () => {
    order.push('forkPipeline');
  }),
  restartNode: vi.fn(async () => {
    order.push('restartNode');
  }),
  setNodeStatus: vi.fn(async () => {
    order.push('setNodeStatus');
  }),
  viewPipeline: vi.fn(async () => {
    order.push('viewPipeline');
  }),
  refreshList: vi.fn(async () => {
    order.push('refreshList');
  }),
};

const fakeDesign = {
  designModeActive: false,
  confirmDiscardDesignChanges: vi.fn(() => true),
  exitDesignMode: vi.fn(),
};

vi.mock('../../../stores/pipelines', () => ({
  usePipelinesStore: () => fakeStore,
}));
vi.mock('../../../stores/pipelineDesign', () => ({
  usePipelineDesignStore: () => fakeDesign,
}));

import { useSurgeryActions } from './useSurgeryActions';
import { useSurgeryController } from './useSurgeryController';

const stop = vi.fn(() => order.push('pause'));
const start = vi.fn(() => order.push('resume'));

beforeEach(() => {
  order.length = 0;
  fakeStore.selectedPipelineId = 'p1';
  fakeDesign.designModeActive = false;
  vi.clearAllMocks();
  useSurgeryController().reset();
  useSurgeryController().registerPollControl({ stop, start });
});

describe('runSurgery cycle ordering', () => {
  it('rewind: pause → mutate → view → refresh → resume', async () => {
    const actions = useSurgeryActions();
    const result = await actions.rewind(['a', 'b']);
    expect(result.ok).toBe(true);
    expect(order).toEqual(['pause', 'rewindNodes', 'viewPipeline', 'refreshList', 'resume']);
  });

  it('batch rewind issues ONE rewindNodes call with ALL ids', async () => {
    const actions = useSurgeryActions();
    await actions.rewind(['a', 'b', 'c']);
    expect(fakeStore.rewindNodes).toHaveBeenCalledTimes(1);
    expect(fakeStore.rewindNodes).toHaveBeenCalledWith('p1', ['a', 'b', 'c']);
  });

  it('resumes polling and re-views even when the mutation throws', async () => {
    fakeStore.rewindNodes.mockRejectedValueOnce(new Error('lease held'));
    const actions = useSurgeryActions();
    const result = await actions.rewind(['a']);
    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/lease held/);
    // pause first, a re-view after the failure, and resume in finally.
    expect(order[0]).toBe('pause');
    expect(order).toContain('viewPipeline');
    expect(order[order.length - 1]).toBe('resume');
    expect(start).toHaveBeenCalledTimes(1);
  });
});

describe('per-op store mapping', () => {
  it('fork forwards an optional new session id (blank → undefined)', async () => {
    const actions = useSurgeryActions();
    await actions.fork('n1', '');
    expect(fakeStore.forkPipeline).toHaveBeenCalledWith('p1', 'n1', undefined);
  });

  it('abandon maps to setNodeStatus(..., "abandoned")', async () => {
    const actions = useSurgeryActions();
    await actions.abandonNode('n1');
    expect(fakeStore.setNodeStatus).toHaveBeenCalledWith('p1', 'n1', 'abandoned', undefined);
  });

  it('fails fast with no selected pipeline (no mutation, no pause)', async () => {
    fakeStore.selectedPipelineId = null;
    const actions = useSurgeryActions();
    const result = await actions.rewind(['a']);
    expect(result.ok).toBe(false);
    expect(fakeStore.rewindNodes).not.toHaveBeenCalled();
    expect(stop).not.toHaveBeenCalled();
  });
});

describe('cross-pipeline leak guard (Defect 3)', () => {
  it('refuses to mutate when the selection changed since the surface opened', async () => {
    // Surface opened against p1 (captures targetSessionId), then the user
    // switched the live selection to p2 before submitting.
    useSurgeryController().openDialog('graft');
    expect(useSurgeryController().targetSessionId.value).toBe('p1');
    fakeStore.selectedPipelineId = 'p2';

    const actions = useSurgeryActions();
    const result = await actions.rewind(['a']);

    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/selection changed/i);
    expect(fakeStore.rewindNodes).not.toHaveBeenCalled();
    expect(stop).not.toHaveBeenCalled();
  });

  it('allows the mutation when the captured target still matches', async () => {
    useSurgeryController().openDialog('graft');
    const actions = useSurgeryActions();
    const result = await actions.rewind(['a']);
    expect(result.ok).toBe(true);
    expect(fakeStore.rewindNodes).toHaveBeenCalledTimes(1);
  });
});
