// T-328.2: contract — after store.loadPipeline succeeds, currentView is
// truthy and the PipelineView template-gallery gate evaluates to false.
//
// PipelineView.vue gates the launcher with:
//   showFullScreenEmpty     = designMode && !currentView && nodes.length === 0
//   showBrowseExistingEntry = !designMode && !currentView && nodes.length === 0
//   showTemplateGallery     = showFullScreenEmpty || showBrowseExistingEntry
//
// This test mirrors that computed using the post-load store state so that any
// future regression that prevents `currentView` from being set (or fails to
// flip `designMode` off) immediately breaks the contract.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import type { PipelineView } from '../../lib/pipeline-types-view';
import { usePipelineStore } from '../pipeline';

const connected = ref(true);
const error = ref<string | null>(null);

const viewPipeline = vi.fn<() => Promise<PipelineView>>();
const startPolling = vi.fn(() => vi.fn());

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    connected,
    reconnecting: ref(false),
    error,
    callTool: vi.fn(),
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    createPipeline: vi.fn(),
    viewPipeline,
    runPipeline: vi.fn(),
    pausePipeline: vi.fn(),
    resumePipeline: vi.fn(),
    cancelPipeline: vi.fn(),
    deletePipeline: vi.fn(),
    graftPipeline: vi.fn(),
    rewindPipeline: vi.fn(),
    forkPipeline: vi.fn(),
    joinPipeline: vi.fn(),
    restartNode: vi.fn(),
    updateNodeStatus: vi.fn(),
    dispatchPipeline: vi.fn(),
    listWorkflows: vi.fn(async () => []),
    listWorkflowSummaries: vi.fn(async () => []),
    readWorkflowDefinition: vi.fn(),
    listTools: vi.fn(async () => []),
    startPolling,
    diffViews: vi.fn(() => []),
  }),
}));

function makeView(): PipelineView {
  return {
    pipelineId: 'pipeline-1779111443919',
    status: 'active',
    running: true,
    ready: [],
    active: ['workflow-a'],
    completed: [],
    mermaid: 'graph TD;',
    edges: [
      { source: 'workflow-a', target: 'workflow-b', type: 'success' },
    ],
    nodes: [
      {
        nodeId: 'workflow-a',
        sessionId: 'session-a',
        status: 'active',
        attempts: 0,
        type: 'workflow',
        workflowId: 'scientific-method',
      },
      {
        nodeId: 'workflow-b',
        sessionId: 'session-b',
        status: 'pending',
        attempts: 0,
        type: 'workflow',
        workflowId: 'peer-review',
      },
    ],
  };
}

describe('T-328.2 pipeline.loadPipeline + view-switch contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('sets currentView and flips the TemplateGallery gate off after a successful load', async () => {
    viewPipeline.mockResolvedValueOnce(makeView());
    const store = usePipelineStore();

    // Pre-conditions match a fresh "browse existing" launcher state:
    //   designMode=false (the user has not entered design mode),
    //   currentView=null, localDefinition.nodes is empty → gallery is shown.
    store.designMode = false;
    expect(store.currentView).toBeNull();
    expect(store.localDefinition.nodes).toHaveLength(0);

    const showBrowseExistingEntryBefore =
      !store.designMode && !store.currentView && store.localDefinition.nodes.length === 0;
    expect(showBrowseExistingEntryBefore).toBe(true);

    const ok = await store.loadPipeline('pipeline-1779111443919');

    expect(ok).toBe(true);
    expect(store.currentPipelineId).toBe('pipeline-1779111443919');
    expect(store.currentView).not.toBeNull();
    expect(store.currentView?.pipelineId).toBe('pipeline-1779111443919');
    expect(store.currentView?.nodes).toHaveLength(2);
    expect(store.localDefinition.nodes).toHaveLength(2);
    expect(store.designMode).toBe(false);

    // Post-conditions match the PipelineView computed gates with currentView truthy.
    const showFullScreenEmpty =
      store.designMode && !store.currentView && store.localDefinition.nodes.length === 0;
    const showBrowseExistingEntry =
      !store.designMode && !store.currentView && store.localDefinition.nodes.length === 0;
    const showTemplateGallery = showFullScreenEmpty || showBrowseExistingEntry;
    expect(showTemplateGallery).toBe(false);
  });
});
