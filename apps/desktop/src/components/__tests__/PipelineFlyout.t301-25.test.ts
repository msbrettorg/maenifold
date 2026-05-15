// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import PipelineFlyout from '../PipelineFlyout.vue';
import { usePipelineStore } from '../../stores/pipeline';

const mcpState = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    listPipelines: vi.fn(),
    connected: vue.ref(true),
    error: vue.ref<string | null>(null),
    reconnecting: vue.ref(false),
  };
});

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected: mcpState.connected,
    error: mcpState.error,
    listPipelines: mcpState.listPipelines,
    reconnecting: mcpState.reconnecting,
  }),
}));

vi.mock('../../composables/useMcpResources', () => ({
  McpCapabilityUnsupportedError: class McpCapabilityUnsupportedError extends Error {},
  useMcpResources: () => ({
    readResource: vi.fn(async () => {
      throw new Error('resource unavailable');
    }),
    subscribeResource: vi.fn(),
  }),
}));

vi.mock('lucide-vue-next', () => ({
  AlertTriangle: { template: '<span />' },
  Plus: { template: '<span />' },
  RefreshCw: { template: '<span />' },
  X: { template: '<span />' },
}));

function pipelineItem(overrides: Record<string, unknown>) {
  return {
    pipelineId: 'pipeline-123',
    status: 'paused',
    nodeCount: 1,
    completedCount: 0,
    description: null,
    runs: [],
    ...overrides,
  };
}

describe('PipelineFlyout T-301.25 display name', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    setActivePinia(createPinia());
    mcpState.listPipelines.mockResolvedValue({
      pipelines: [
        pipelineItem({ pipelineId: 'pipeline-123', name: 'user-pipeline' }),
        pipelineItem({ pipelineId: 'pipeline-456', name: '' }),
      ],
    });
  });

  it('PipelineFlyout_t301_25_shows_user_name_and_falls_back_to_session_id', async () => {
    const store = usePipelineStore();
    await store.refreshList();
    const wrapper = mount(PipelineFlyout, {
      attachTo: document.body,
      props: {
        connectionError: '',
        showErrorState: false,
        showLoadingState: false,
      },
    });

    await flushPromises();

    expect(mcpState.listPipelines).toHaveBeenCalled();
    const text = document.body.textContent ?? wrapper.text();
    expect(text).toContain('user-pipeline');
    expect(text).toContain('pipeline-456');
  });
});
