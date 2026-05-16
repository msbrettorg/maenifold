// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { ref } from 'vue';
import PipelineSidebar from '../PipelineSidebar.vue';
import { usePipelineStore } from '../../stores/pipeline';
import type { PipelineListItem } from '../../lib/pipeline-types-view';

vi.mock('../../composables/useMcpResources', () => ({
  McpCapabilityUnsupportedError: class extends Error {},
  useMcpResources: () => ({
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
    unsubscribeResource: vi.fn(),
  }),
}));

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    connected: ref(false),
    error: ref(null),
    reconnecting: ref(false),
    callTool: vi.fn(),
    connect: vi.fn(),
    disconnect: vi.fn(),
    listTools: vi.fn(),
    onDisconnect: vi.fn(() => vi.fn()),
    onNotification: vi.fn(() => vi.fn()),
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
    supportsResourceSubscribe: vi.fn(() => false),
    unsubscribeResource: vi.fn(),
  }),
  usePipelineMcp: () => ({
    connected: ref(false),
    error: ref(null),
    reconnecting: ref(false),
    callPipeline: vi.fn(),
    callTool: vi.fn(),
    connect: vi.fn(),
    disconnect: vi.fn(),
    diffViews: vi.fn(() => []),
    listAssets: vi.fn(async () => []),
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    listTools: vi.fn(async () => []),
    listWorkflows: vi.fn(async () => []),
    onDisconnect: vi.fn(() => vi.fn()),
    onNotification: vi.fn(() => vi.fn()),
    startPolling: vi.fn(() => vi.fn()),
    supportsResourceSubscribe: vi.fn(() => false),
  }),
}));

function pipelineItem(index: number): PipelineListItem {
  return {
    pipelineId: `pipeline-${index.toString().padStart(2, '0')}`,
    status: 'paused',
    nodeCount: 4,
    completedCount: 2,
    runs: [],
  };
}

describe('PipelineSidebar', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    document.body.innerHTML = '';
  });

  it('keeps overflowing pipeline items in a vertical-only scroll region', () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = usePipelineStore();
    store.pipelines = Array.from({ length: 55 }, (_, index) => pipelineItem(index + 1));

    const wrapper = mount(PipelineSidebar, {
      attachTo: document.body,
      global: {
        plugins: [pinia],
      },
    });

    const scrollRegion = wrapper.get('[data-testid="pipeline-sidebar-scroll"]');
    const classes = scrollRegion.classes();

    expect(classes).toContain('min-h-0');
    expect(classes).toContain('flex-1');
    expect(classes).toContain('overflow-y-auto');
    expect(classes).toContain('overflow-x-hidden');
    expect(wrapper.findAll('[data-pipeline-item]')).toHaveLength(55);
  });
});
