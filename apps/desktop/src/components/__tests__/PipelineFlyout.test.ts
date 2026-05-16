// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { defineComponent } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import PipelineFlyout from '../PipelineFlyout.vue';
import ConfirmDialog from '../ConfirmDialog.vue';
import { usePipelineStore } from '../../stores/pipeline';
import type { PipelineListItem } from '../../lib/pipeline-types-view';

const toast = vi.hoisted(() => ({
  error: vi.fn(),
  success: vi.fn(),
  info: vi.fn(),
}));

const deletePipeline = vi.hoisted(() => vi.fn());
const listPipelines = vi.hoisted(() => vi.fn(async () => ({ pipelines: [] })));
const mcpState = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    connected: vue.ref(false),
    reconnecting: vue.ref(false),
    error: vue.ref<string | null>(null),
  };
});

vi.mock('vue-sonner', () => ({ toast }));

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    connected: mcpState.connected,
    reconnecting: mcpState.reconnecting,
    error: mcpState.error,
    callTool: vi.fn(),
    callPipeline: vi.fn(),
    listPipelines,
    deletePipeline,
    startPolling: vi.fn(() => vi.fn()),
    diffViews: vi.fn(() => []),
    listWorkflows: vi.fn(async () => []),
    listWorkflowSummaries: vi.fn(async () => []),
    readWorkflowDefinition: vi.fn(async () => ({})),
    listTools: vi.fn(async () => []),
  }),
}));

vi.mock('../../composables/useMcpResources', () => ({
  McpCapabilityUnsupportedError: class extends Error {},
  useMcpResources: () => ({
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
  }),
}));

function pipelineItem(pipelineId: string): PipelineListItem {
  return {
    pipelineId,
    status: 'completed',
    nodeCount: 1,
    completedCount: 1,
    description: 'Delete me',
    runs: [],
  };
}

function mountFlyout() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = usePipelineStore();
  store.pipelines = [pipelineItem('pipeline-1735689600000-t301-27-flyout')];
  const Host = defineComponent({
    components: { PipelineFlyout, ConfirmDialog },
    template: `
      <PipelineFlyout
        connection-error=""
        :show-error-state="false"
        :show-loading-state="false"
      />
      <ConfirmDialog />
    `,
  });
  const wrapper = mount(Host, {
    attachTo: document.body,
    global: { plugins: [pinia] },
  });
  return { store, wrapper };
}

describe('PipelineFlyout delete flow', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
    mcpState.connected.value = false;
    mcpState.reconnecting.value = false;
    mcpState.error.value = null;
  });

  it('delete_t301_27_opens_confirm_dialog_then_toasts_success_info_and_refreshes_list', async () => {
    deletePipeline.mockResolvedValueOnce({
      message: 'Pipeline moved to quarantine for 30 days.',
      defaultedRetention: true,
      deletedFiles: ['/memory/pipelines/2025/01/01/pipeline.md'],
    });
    mountFlyout();
    await flushPromises();

    const deleteButton = document.body.querySelector<HTMLButtonElement>(
      'button[aria-label="Delete pipeline"]',
    );
    expect(deleteButton).toBeTruthy();
    deleteButton?.click();
    await flushPromises();

    expect(document.body.textContent).toContain(
      'Delete pipeline "pipeline-1735689600000-t301-27-flyout"?',
    );
    expect(deletePipeline).not.toHaveBeenCalled();

    const continueButton = Array.from(document.body.querySelectorAll('button'))
      .find((button) => button.textContent?.includes('Continue'));
    expect(continueButton).toBeTruthy();
    continueButton?.click();
    await flushPromises();

    expect(deletePipeline).toHaveBeenCalledWith('pipeline-1735689600000-t301-27-flyout');
    expect(listPipelines).toHaveBeenCalled();
    expect(toast.success).toHaveBeenCalledWith('Pipeline moved to quarantine for 30 days.');
    expect(toast.info).toHaveBeenCalledWith(
      expect.stringContaining('default 30-day quarantine retention'),
    );
  });
});
