import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import { usePipelineStore } from '../pipeline';
import type { PipelineDefinition, PipelineView } from '../../lib/pipeline-types';

const createPipeline = vi.fn(async () => ({ pipelineId: 'pipeline-created' }));
const updatePipeline = vi.fn(async (sessionId: string) => ({ pipelineId: sessionId }));
const viewPipeline = vi.fn(async (sessionId: string): Promise<PipelineView> => ({
  pipelineId: sessionId,
  status: 'paused',
  running: false,
  ready: [],
  active: [],
  completed: [],
  mermaid: 'graph TD;',
  nodes: [],
  edges: [],
}));
const listPipelines = vi.fn(async () => ({ pipelines: [] }));

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    cancelPipeline: vi.fn(),
    connect: vi.fn(),
    connected: ref(true),
    createPipeline,
    deletePipeline: vi.fn(),
    diffViews: vi.fn(() => []),
    disconnect: vi.fn(),
    dispatchPipeline: vi.fn(),
    error: ref(null),
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
    restartNode: vi.fn(),
    resumePipeline: vi.fn(),
    rewindPipeline: vi.fn(),
    runPipeline: vi.fn(),
    startPolling: vi.fn(() => vi.fn()),
    updateNodeStatus: vi.fn(),
    updatePipeline,
    viewPipeline,
  }),
}));

const definition: PipelineDefinition = {
  nodes: [{ id: 'node-a', type: 'tool', toolName: 'example.tool' }],
  edges: [],
};

describe('pipeline T-301.26 update save routing', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    createPipeline.mockResolvedValue({ pipelineId: 'pipeline-created' });
    updatePipeline.mockImplementation(async (sessionId: string) => ({ pipelineId: sessionId }));
  });

  it('pipeline_t301_26_save_on_existing_pipeline_calls_update_not_create', async () => {
    const store = usePipelineStore();
    store.currentPipelineId = 'pipeline-existing';
    store.localDefinition = definition;

    await store.createPipeline(undefined, { name: 'Renamed', description: 'Updated' });

    expect(updatePipeline).toHaveBeenCalledWith(
      'pipeline-existing',
      expect.objectContaining({ nodes: definition.nodes, edges: definition.edges }),
      expect.objectContaining({ name: 'Renamed', description: 'Updated' }),
    );
    expect(createPipeline).not.toHaveBeenCalled();
  });

  it('pipeline_t301_26_save_on_new_pipeline_calls_create_not_update', async () => {
    const store = usePipelineStore();
    store.currentPipelineId = null;
    store.localDefinition = definition;

    await store.createPipeline(undefined, { name: 'New', description: 'Created' });

    expect(createPipeline).toHaveBeenCalled();
    expect(updatePipeline).not.toHaveBeenCalled();
  });

  it('pipeline_t301_26_save_breadcrumb_unchanged_after_update', async () => {
    const store = usePipelineStore();
    store.currentPipelineId = 'pipeline-existing';
    store.localDefinition = definition;

    await store.createPipeline(undefined, { name: 'Renamed' });

    expect(store.currentPipelineId).toBe('pipeline-existing');
  });
});
