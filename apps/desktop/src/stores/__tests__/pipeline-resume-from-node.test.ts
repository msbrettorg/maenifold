import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { PipelineView } from '../../lib/pipeline-types';
import { usePipelineStore } from '../pipeline';

const mocks = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    connected: vue.ref(true),
    error: vue.ref<string | null>(null),
    reconnecting: vue.ref(false),
    stopPolling: vi.fn(),
    restartNode: vi.fn(async () => ({})),
    runPipeline: vi.fn(async () => ({})),
    viewPipeline: vi.fn<() => Promise<PipelineView>>(),
    startPolling: vi.fn(() => vi.fn()),
    diffViews: vi.fn(() => []),
  };
});

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    connected: mocks.connected,
    reconnecting: mocks.reconnecting,
    error: mocks.error,
    callTool: vi.fn(),
    callPipeline: vi.fn(),
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    createPipeline: vi.fn(async () => ({ pipelineId: 'created' })),
    viewPipeline: mocks.viewPipeline,
    runPipeline: mocks.runPipeline,
    pausePipeline: vi.fn(async () => ({})),
    resumePipeline: vi.fn(async () => ({})),
    cancelPipeline: vi.fn(async () => ({})),
    deletePipeline: vi.fn(async () => ({})),
    graftPipeline: vi.fn(async () => ({})),
    rewindPipeline: vi.fn(async () => ({})),
    forkPipeline: vi.fn(async () => ({})),
    joinPipeline: vi.fn(async () => ({})),
    restartNode: mocks.restartNode,
    updateNodeStatus: vi.fn(async () => ({})),
    dispatchPipeline: vi.fn(async () => ({})),
    listWorkflows: vi.fn(async () => []),
    listWorkflowSummaries: vi.fn(async () => []),
    readWorkflowDefinition: vi.fn(async () => ({})),
    listTools: vi.fn(async () => []),
    listAssets: vi.fn(async () => []),
    startPolling: mocks.startPolling,
    diffViews: mocks.diffViews,
  }),
}));

function createView(overrides?: Partial<PipelineView>): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'stuck',
    running: false,
    ready: [],
    active: [],
    completed: ['node-done'],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'node-done',
        sessionId: 'session-done',
        status: 'completed',
        attempts: 1,
        type: 'tool',
        toolName: 'example.done',
      },
      {
        nodeId: 'node-failed',
        sessionId: 'session-failed',
        status: 'abandoned',
        attempts: 1,
        type: 'tool',
        toolName: 'example.failed',
        failureReason: 'tool failed',
      },
    ],
    ...overrides,
  };
}

describe('pipeline resumeFromNode action', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mocks.connected.value = true;
    mocks.error.value = null;
    mocks.reconnecting.value = false;
    vi.clearAllMocks();
  });

  it('restarts the target node, runs the pipeline, refreshes view, and starts polling', async () => {
    const runningView = createView({
      status: 'active',
      running: true,
      active: ['node-failed'],
      nodes: [
        createView().nodes[0]!,
        {
          ...createView().nodes[1]!,
          status: 'active',
          attempts: 2,
          failureReason: null,
        },
      ],
    });
    mocks.viewPipeline.mockResolvedValueOnce(runningView);
    const store = usePipelineStore();
    store.designMode = false;
    store.currentPipelineId = 'pipeline-1';
    store.currentView = createView();
    store.localDefinition = { nodes: [], edges: [] };

    await expect(store.resumeFromNode('node-failed')).resolves.toBeUndefined();

    expect(mocks.restartNode).toHaveBeenCalledWith('pipeline-1', 'node-failed');
    expect(mocks.runPipeline).toHaveBeenCalledWith('pipeline-1');
    expect(mocks.startPolling).toHaveBeenCalledWith('pipeline-1', expect.any(Function), 3000);
    expect(store.currentView?.running).toBe(true);
    expect(store.currentView?.active).toEqual(['node-failed']);
  });
});
