import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import { usePipelineStore } from '../pipeline';
import type { PipelineView } from '../../lib/pipeline-types-view';

const connected = ref(true);
const error = ref<string | null>(null);
const createPipeline = vi.fn(async () => ({ pipelineId: 'pipeline-created' }));
const listPipelines = vi.fn(async () => ({ pipelines: [{ pipelineId: 'pipeline-created', status: 'paused' }] }));
const viewPipeline = vi.fn(async (): Promise<PipelineView> => ({
  pipelineId: 'pipeline-created',
  status: 'paused',
  running: false,
  ready: [],
  active: [],
  completed: [],
  mermaid: 'graph TD;',
  nodes: [],
  edges: [],
}));

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    cancelPipeline: vi.fn(),
    connect: vi.fn(),
    connected,
    deletePipeline: vi.fn(),
    diffViews: vi.fn(() => []),
    disconnect: vi.fn(),
    dispatchPipeline: vi.fn(),
    error,
    forkPipeline: vi.fn(),
    graftPipeline: vi.fn(),
    joinPipeline: vi.fn(),
    listAssets: vi.fn(async () => []),
    listPipelines,
    listTools: vi.fn(async () => []),
    listWorkflowSummaries: vi.fn(async () => []),
    listWorkflows: vi.fn(async () => []),
    pausePipeline: vi.fn(),
    readWorkflowDefinition: vi.fn(),
    reconnecting: ref(false),
    resumePipeline: vi.fn(),
    rewindPipeline: vi.fn(),
    runPipeline: vi.fn(),
    startPolling: vi.fn(() => vi.fn()),
    updateNodeStatus: vi.fn(),
    restartNode: vi.fn(),
    viewPipeline,
    createPipeline,
  }),
}));

describe('pipeline design save state', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setActivePinia(createPinia());
    vi.clearAllMocks();
    createPipeline.mockResolvedValue({ pipelineId: 'pipeline-created' });
    error.value = null;
  });

  it('transitions dirty draft save from saving to saved to idle', async () => {
    const store = usePipelineStore();
    store.designMode = true;
    store.localDefinition = { nodes: [{ id: 'draft-node', type: 'tool', toolName: 'example.tool' }], edges: [] };

    const save = store.createPipeline();
    expect(store.savingState).toBe('saving');

    await save;
    expect(store.designMode).toBe(false);
    expect(store.currentPipelineId).toBe('pipeline-created');
    expect(store.savingState).toBe('saved');

    vi.advanceTimersByTime(2000);
    expect(store.savingState).toBe('idle');
  });

  it('transitions failed saves to error and then idle', async () => {
    createPipeline.mockRejectedValueOnce(new Error('create failed'));
    const store = usePipelineStore();
    store.designMode = true;
    store.localDefinition = { nodes: [{ id: 'draft-node', type: 'tool', toolName: 'example.tool' }], edges: [] };

    await expect(store.createPipeline()).resolves.toBeUndefined();
    expect(store.designMode).toBe(true);
    expect(store.savingState).toBe('error');

    vi.advanceTimersByTime(2000);
    expect(store.savingState).toBe('idle');
  });
});
