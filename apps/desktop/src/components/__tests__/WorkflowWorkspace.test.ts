// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, ref } from 'vue';
import WorkflowWorkspace from '../WorkflowWorkspace.vue';
import { usePipelineStore } from '../../stores/pipeline';
import { useWorkflowLibraryStore } from '../../stores/workflow-library';

const guard = vi.hoisted(() => ({
  markDirty: vi.fn(),
  markClean: vi.fn(),
  confirmIfDirty: vi.fn(async () => true),
  attachBeforeUnload: vi.fn(() => vi.fn()),
}));
let pinia: ReturnType<typeof createPinia>;

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    connected: ref(true),
    connect: vi.fn(),
    callTool: vi.fn(),
    error: ref(null),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
    unsubscribeResource: vi.fn(),
    supportsResourceSubscribe: vi.fn(() => true),
  }),
  usePipelineMcp: () => ({
    connected: ref(true),
    connect: vi.fn(),
    callTool: vi.fn(),
    error: ref(null),
    readWorkflowDefinition: vi.fn(),
  }),
}));

vi.mock('../../composables/useUnsavedChangesGuard', () => ({
  useUnsavedChangesGuard: vi.fn(() => ({
    isDirty: ref(false),
    markDirty: guard.markDirty,
    markClean: guard.markClean,
    confirmIfDirty: guard.confirmIfDirty,
    attachBeforeUnload: guard.attachBeforeUnload,
  })),
}));

function workflow(name = 'Workflow A') {
  return {
    id: 'workflow-a',
    name,
    emoji: '🧪',
    description: 'A workflow',
    triggers: [],
    sequentialThinkingEnabled: false,
    multiAgent: false,
    steps: [{ id: 'step-1', name: 'Step 1', description: 'Do it', requiresSequentialThinking: false }],
  };
}

function summary(id: string, name: string, description: string) {
  return { id, name, description, emoji: '🧪', multiAgent: false };
}

const WorkflowStepInspectorStub = defineComponent({
  setup() {
    const store = useWorkflowLibraryStore();
    return { store };
  },
  template: '<input data-testid="workflow-name" @input="store.updateWorkflowName($event.target.value)" />',
});

const WorkflowStepTimelineStub = defineComponent({
  emits: ['save'],
  template: '<button data-testid="workflow-save" type="button" @click="$emit(\'save\')">Save</button>',
});

const WorkflowViewStub = defineComponent({
  props: { filterWorkflowId: { type: String, required: false, default: null } },
  template: '<section data-testid="workflow-view-stub">{{ filterWorkflowId }}</section>',
});

describe('WorkflowWorkspace unsaved changes guard', () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    const pipelineStore = usePipelineStore();
    pipelineStore.connected = true;
    vi.spyOn(pipelineStore, 'connect').mockResolvedValue();
    vi.spyOn(pipelineStore, 'listWorkflowSummaries').mockResolvedValue([
      summary('workflow-a', 'Workflow A', 'A workflow'),
      summary('workflow-b', 'Workflow B', 'B workflow'),
    ]);
    guard.markDirty.mockClear();
    guard.markClean.mockClear();
    guard.confirmIfDirty.mockClear();
    guard.attachBeforeUnload.mockClear();
    guard.confirmIfDirty.mockResolvedValue(true);
  });

  it('uses the shared guard for draft edits, save success, and workflow selection changes', async () => {
    const store = useWorkflowLibraryStore();
    store.currentId = 'workflow-a';
    store.currentDefinition = workflow();
    store.draftDefinition = workflow();
    store.rawJson = workflow() as never;
    store.summaries = [
      summary('workflow-a', 'Workflow A', 'A workflow'),
      summary('workflow-b', 'Workflow B', 'B workflow'),
    ];
    vi.spyOn(store, 'fetchSummaries').mockResolvedValue();
    vi.spyOn(store, 'ensureVisibleWorkflowSelected').mockResolvedValue();
    vi.spyOn(store, 'saveDraft').mockResolvedValue(true);

    const wrapper = mount(WorkflowWorkspace, {
      global: {
        plugins: [pinia],
        stubs: {
          WorkflowBrowser: true,
          WorkflowLiveProgress: true,
          WorkflowStepInspector: WorkflowStepInspectorStub,
          WorkflowStepTimeline: WorkflowStepTimelineStub,
          WorkflowView: WorkflowViewStub,
        },
      },
    });
    await flushPromises();

    await wrapper.get('[data-testid="workflow-name"]').setValue('Workflow edited');
    expect(guard.markDirty).toHaveBeenCalled();

    await wrapper.get('[data-testid="workflow-save"]').trigger('click');
    await flushPromises();
    expect(guard.markClean).toHaveBeenCalled();

    store.pendingWorkflowId = 'workflow-b';
    await flushPromises();
    expect(guard.confirmIfDirty).toHaveBeenCalled();
  });

  it('renders WorkflowView for the selected workflow', async () => {
    const store = useWorkflowLibraryStore();
    store.currentId = 'workflow-a';
    store.currentDefinition = workflow();
    store.summaries = [
      summary('workflow-a', 'Workflow A', 'A workflow'),
      summary('workflow-b', 'Workflow B', 'B workflow'),
    ];
    vi.spyOn(store, 'fetchSummaries').mockResolvedValue();
    vi.spyOn(store, 'ensureVisibleWorkflowSelected').mockResolvedValue();

    const wrapper = mount(WorkflowWorkspace, {
      global: {
        plugins: [pinia],
        stubs: {
          WorkflowBrowser: true,
          WorkflowLiveProgress: true,
          WorkflowStepInspector: WorkflowStepInspectorStub,
          WorkflowStepTimeline: WorkflowStepTimelineStub,
          WorkflowView: WorkflowViewStub,
        },
      },
    });
    await flushPromises();

    const workflowView = wrapper.get('[data-testid="workflow-view-stub"]');
    expect(workflowView.text()).toContain('workflow-a');
  });
});
