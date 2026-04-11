import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import type { PipelineView } from '../../lib/pipeline-types';
import { usePipelineStore } from '../pipeline';

const connected = ref(true);
const error = ref<string | null>(null);
const connect = vi.fn(async () => {});
const disconnect = vi.fn();
const listPipelines = vi.fn(async () => ({ pipelines: [] }));
const createPipeline = vi.fn(async () => ({ pipelineId: 'pipeline-created' }));
const viewPipeline = vi.fn<() => Promise<PipelineView>>();
const runPipeline = vi.fn(async () => ({}));
const pausePipeline = vi.fn(async () => ({}));
const resumePipeline = vi.fn(async () => ({}));
const cancelPipeline = vi.fn(async () => ({}));
const deletePipeline = vi.fn(async () => ({}));
const graftPipeline = vi.fn(async () => ({}));
const rewindPipeline = vi.fn(async () => ({}));
const forkPipeline = vi.fn(async () => ({}));
const joinPipeline = vi.fn(async () => ({}));
const restartNode = vi.fn(async () => ({}));
const updateNodeStatus = vi.fn(async () => ({}));
const dispatchPipeline = vi.fn(async () => ({}));
const listWorkflows = vi.fn(async () => []);
const listWorkflowSummaries = vi.fn(async () => []);
const readWorkflowDefinition = vi.fn(async () => ({}));
const listTools = vi.fn(async () => []);
const stopPolling = vi.fn();
let pollingCallback: ((view: PipelineView) => void) | null = null;

const diffViews = vi.fn((prev: PipelineView | null | undefined, curr: PipelineView | null | undefined) => {
  if (!prev?.nodes || !curr?.nodes) {
    return [];
  }

  const previousStatuses = new Map(prev.nodes.map((node) => [node.nodeId, node.status]));
  return curr.nodes.flatMap((node) => {
    const previousStatus = previousStatuses.get(node.nodeId);
    return previousStatus != null && previousStatus !== node.status
      ? [{ nodeId: node.nodeId, from: previousStatus, to: node.status, timestamp: Date.now() }]
      : [];
  });
});

const startPolling = vi.fn((_pipelineId: string, onView: (view: PipelineView) => void) => {
  pollingCallback = onView;
  return stopPolling;
});

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connect,
    disconnect,
    connected,
    reconnecting: ref(false),
    error,
    callTool: vi.fn(),
    callPipeline: vi.fn(),
    listPipelines,
    createPipeline,
    viewPipeline,
    runPipeline,
    pausePipeline,
    resumePipeline,
    cancelPipeline,
    deletePipeline,
    graftPipeline,
    rewindPipeline,
    forkPipeline,
    joinPipeline,
    restartNode,
    updateNodeStatus,
    dispatchPipeline,
    listWorkflows,
    listWorkflowSummaries,
    readWorkflowDefinition,
    listTools,
    startPolling,
    diffViews,
  }),
}));

function createView(overrides?: Partial<PipelineView>): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: ['node-active'],
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
        output: 'done output',
      },
      {
        nodeId: 'node-active',
        sessionId: 'session-active',
        status: 'active',
        attempts: 1,
        type: 'tool',
        toolName: 'example.active',
      },
      {
        nodeId: 'node-pending',
        sessionId: 'session-pending',
        status: 'pending',
        attempts: 0,
        type: 'tool',
        toolName: 'example.pending',
      },
    ],
    ...overrides,
  };
}

describe('usePipelineStore pipeline loading', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    connected.value = true;
    error.value = null;
    pollingCallback = null;
    vi.clearAllMocks();
  });

  it('loads a pipeline, seeds hydrated events, and resumes polling for non-terminal views', async () => {
    const initialView = createView();
    viewPipeline.mockResolvedValueOnce(initialView);
    const store = usePipelineStore();

    await store.loadPipeline('pipeline-1');

    expect(store.currentPipelineId).toBe('pipeline-1');
    expect(store.designMode).toBe(false);
    expect(store.eventLog).toEqual([
      expect.objectContaining({ nodeId: 'node-done', from: 'loaded', to: 'completed', preview: 'done output' }),
      expect.objectContaining({ nodeId: 'node-active', from: 'loaded', to: 'active' }),
    ]);
    expect(startPolling).toHaveBeenCalledWith('pipeline-1', expect.any(Function), 3000);
    expect(pollingCallback).toBeTypeOf('function');
  });

  it('appends only real polling diffs and does not duplicate hydrated entries', async () => {
    const initialView = createView();
    viewPipeline.mockResolvedValueOnce(initialView);
    const store = usePipelineStore();

    await store.loadPipeline('pipeline-1');
    const hydratedEntries = [...store.eventLog];

    pollingCallback?.(createView());
    expect(store.eventLog).toEqual(hydratedEntries);

    pollingCallback?.(createView({
      active: [],
      completed: ['node-done', 'node-pending'],
      nodes: [
        initialView.nodes[0]!,
        {
          ...initialView.nodes[1]!,
          status: 'active',
        },
        {
          ...initialView.nodes[2]!,
          status: 'completed',
          attempts: 1,
          output: 'finished later',
        },
      ],
    }));

    expect(store.eventLog).toHaveLength(hydratedEntries.length + 1);
    expect(store.eventLog[store.eventLog.length - 1]).toEqual(expect.objectContaining({
      nodeId: 'node-pending',
      from: 'pending',
      to: 'completed',
    }));

    pollingCallback?.(createView({
      active: [],
      completed: ['node-done', 'node-pending'],
      nodes: [
        initialView.nodes[0]!,
        {
          ...initialView.nodes[1]!,
          status: 'active',
        },
        {
          ...initialView.nodes[2]!,
          status: 'completed',
          attempts: 1,
          output: 'finished later',
        },
      ],
    }));

    expect(store.eventLog).toHaveLength(hydratedEntries.length + 1);
  });
});
