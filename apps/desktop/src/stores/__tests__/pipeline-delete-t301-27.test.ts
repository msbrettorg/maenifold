import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import type { PipelineView } from '../../lib/pipeline-types';
import { usePipelineStore } from '../pipeline';

const toast = vi.hoisted(() => ({
  error: vi.fn(),
  success: vi.fn(),
  info: vi.fn(),
}));

const connected = ref(true);
const error = ref<string | null>(null);
const listPipelines = vi.fn(async () => ({ pipelines: [] }));
const deletePipeline = vi.fn(async () => ({}));
const stopPolling = vi.fn();

vi.mock('vue-sonner', () => ({ toast }));

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    connected,
    reconnecting: ref(false),
    error,
    callTool: vi.fn(),
    callPipeline: vi.fn(),
    listPipelines,
    createPipeline: vi.fn(),
    viewPipeline: vi.fn(),
    runPipeline: vi.fn(),
    pausePipeline: vi.fn(),
    resumePipeline: vi.fn(),
    cancelPipeline: vi.fn(),
    deletePipeline,
    graftPipeline: vi.fn(),
    rewindPipeline: vi.fn(),
    forkPipeline: vi.fn(),
    joinPipeline: vi.fn(),
    restartNode: vi.fn(),
    updateNodeStatus: vi.fn(),
    dispatchPipeline: vi.fn(),
    listWorkflows: vi.fn(async () => []),
    listWorkflowSummaries: vi.fn(async () => []),
    readWorkflowDefinition: vi.fn(async () => ({})),
    listTools: vi.fn(async () => []),
    startPolling: vi.fn(() => stopPolling),
    diffViews: vi.fn(() => []),
  }),
}));

function createView(overrides?: Partial<PipelineView>): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: [],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [],
    ...overrides,
  };
}

describe('pipeline delete T-301.27 store contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    connected.value = true;
    error.value = null;
    vi.clearAllMocks();
  });

  it('delete_t301_27_uses_daemon_message_toasts_default_retention_and_refreshes_list', async () => {
    deletePipeline.mockResolvedValueOnce({
      message: 'Pipeline moved to quarantine for 30 days.',
      defaultedRetention: true,
      deletedFiles: ['/memory/pipelines/2025/01/01/pipeline-1.md'],
    });
    listPipelines.mockResolvedValueOnce({ pipelines: [] });
    const store = usePipelineStore();
    store.currentPipelineId = 'pipeline-1';
    store.currentView = createView({ pipelineId: 'pipeline-1' });

    await expect(store.deletePipelineById('pipeline-1')).resolves.toBeUndefined();

    expect(deletePipeline).toHaveBeenCalledWith('pipeline-1');
    expect(listPipelines).toHaveBeenCalled();
    expect(store.currentPipelineId).toBeNull();
    expect(store.successMessage).toBe('Pipeline moved to quarantine for 30 days.');
    expect(toast.success).toHaveBeenCalledWith('Pipeline moved to quarantine for 30 days.');
    expect(toast.info).toHaveBeenCalledWith(
      expect.stringContaining('default 30-day quarantine retention'),
    );
  });
});
