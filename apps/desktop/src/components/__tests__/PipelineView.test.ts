// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import PipelineView from '../PipelineView.vue';
import { usePipelineStore } from '../../stores/pipeline';
import type { PipelineView as PipelineViewModel } from '../../lib/pipeline-types';

const mcpState = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    connected: vue.ref(true),
    error: vue.ref<string | null>(null),
    reconnecting: vue.ref(false),
  };
});

vi.mock('elkjs/lib/elk-api', () => ({
  default: class {
    layout() {
      return Promise.resolve({ children: [], edges: [] });
    }
  },
}));

vi.mock('elkjs/lib/elk-worker.min.js?url', () => ({
  default: 'mock-elk-worker.js',
}));

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    callTool: vi.fn(),
    connect: vi.fn(),
    connected: mcpState.connected,
    disconnect: vi.fn(),
    error: mcpState.error,
    listTools: vi.fn(),
    onDisconnect: vi.fn(() => vi.fn()),
    onNotification: vi.fn(() => vi.fn()),
    readResource: vi.fn(),
    reconnecting: mcpState.reconnecting,
    subscribeResource: vi.fn(),
    supportsResourceSubscribe: vi.fn(() => true),
    unsubscribeResource: vi.fn(),
  }),
  usePipelineMcp: () => ({
    callPipeline: vi.fn(),
    callTool: vi.fn(),
    connect: vi.fn(),
    connected: mcpState.connected,
    diffViews: vi.fn(() => []),
    disconnect: vi.fn(),
    error: mcpState.error,
    listAssets: vi.fn(async () => []),
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    listTools: vi.fn(async () => []),
    listWorkflows: vi.fn(async () => []),
    onDisconnect: vi.fn(() => vi.fn()),
    onNotification: vi.fn(() => vi.fn()),
    reconnecting: mcpState.reconnecting,
    supportsResourceSubscribe: vi.fn(() => true),
    startPolling: vi.fn(() => vi.fn()),
  }),
}));

function createView(status: PipelineViewModel['status'] = 'paused', pipelineId = 'pipeline-1'): PipelineViewModel {
  return {
    pipelineId,
    status,
    running: false,
    ready: [],
    active: [],
    completed: [],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [],
  };
}

function mockViewport() {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

function mountPipelineView(setupStore?: (store: ReturnType<typeof usePipelineStore>) => void) {
  mockViewport();
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = usePipelineStore();
  store.designMode = false;
  store.currentPipelineId = 'pipeline-1';
  store.currentView = createView();
  store.localDefinition = { nodes: [], edges: [] };
  setupStore?.(store);
  vi.spyOn(store, 'connect').mockResolvedValue();
  vi.spyOn(store, 'disconnect').mockImplementation(() => undefined);
  vi.spyOn(store, 'refreshList').mockResolvedValue();
  vi.spyOn(store, 'refreshWorkflows').mockResolvedValue();
  vi.spyOn(store, 'refreshTools').mockResolvedValue();

  const wrapper = mount(PipelineView, {
    attachTo: document.body,
    global: {
      plugins: [pinia],
      stubs: {
        DesignNodeConfigModal: true,
        GraftNodesDialog: true,
        JoinPipelineDialog: true,
        PipelineActionBar: true,
        PipelineComposer: { props: ['designMode'], template: '<div data-testid="composer" :data-design-mode="String(designMode)"><slot name="toolbar-actions" /></div>' },
        PipelineFlyout: true,
        PipelineIconRail: true,
        PipelineRunner: {
          props: ['collapsed'],
          template: '<div data-testid="runner" :data-collapsed="String(collapsed)" />',
        },
        PipelineSidebar: true,
        PromotionDialog: true,
        ProvenanceBreadcrumbs: true,
        SemanticZoomOverlay: true,
        TemplateGallery: true,
      },
    },
  });

  return { store, wrapper };
}

function runnerToggle(wrapper: ReturnType<typeof mount>) {
  return wrapper.get('button[aria-label="Toggle runner"]');
}

function designToggle(wrapper: ReturnType<typeof mount>) {
  return wrapper.get('button[aria-label="Toggle pipeline design mode"]');
}

async function expectRunnerCollapsed(wrapper: ReturnType<typeof mount>) {
  await nextTick();
  expect(runnerToggle(wrapper).attributes('aria-expanded')).toBe('false');
  expect(wrapper.get('[data-testid="runner"]').attributes('data-collapsed')).toBe('true');
}

async function expectRunnerExpanded(wrapper: ReturnType<typeof mount>) {
  await nextTick();
  expect(runnerToggle(wrapper).attributes('aria-expanded')).toBe('true');
  expect(wrapper.get('[data-testid="runner"]').attributes('data-collapsed')).toBe('false');
}

async function settleRunnerWatchers() {
  await flushPromises();
  await nextTick();
  await nextTick();
}

beforeEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
  mcpState.connected.value = true;
  mcpState.error.value = null;
  mcpState.reconnecting.value = false;
});

describe('PipelineView design mode toggle', () => {
  it('starts in viewer mode and toggles the composer design controls', async () => {
    const { store, wrapper } = mountPipelineView();

    await flushPromises();

    expect(store.designMode).toBe(false);
    expect(wrapper.get('[data-testid="composer"]').attributes('data-design-mode')).toBe('false');
    expect(designToggle(wrapper).text()).toContain('Design');
    expect(designToggle(wrapper).attributes('aria-pressed')).toBe('false');

    await designToggle(wrapper).trigger('click');
    await nextTick();

    expect(store.designMode).toBe(true);
    expect(wrapper.get('[data-testid="composer"]').attributes('data-design-mode')).toBe('true');
    expect(designToggle(wrapper).text()).toContain('Done');
    expect(designToggle(wrapper).attributes('aria-pressed')).toBe('true');

    await designToggle(wrapper).trigger('click');
    await nextTick();

    expect(store.designMode).toBe(false);
    expect(wrapper.get('[data-testid="composer"]').attributes('data-design-mode')).toBe('false');
  });

  it('resets to viewer mode when switching pipelines', async () => {
    const { store, wrapper } = mountPipelineView();

    await flushPromises();
    await designToggle(wrapper).trigger('click');
    await nextTick();
    expect(store.designMode).toBe(true);

    store.currentPipelineId = 'pipeline-2';
    store.currentView = createView('paused', 'pipeline-2');
    await settleRunnerWatchers();

    expect(store.designMode).toBe(false);
    expect(designToggle(wrapper).text()).toContain('Design');
  });
});

describe('PipelineView runner expansion state', () => {
  it('starts collapsed when mounted with a pre-existing pipeline error', async () => {
    mcpState.error.value = 'stale connection failure';
    const { wrapper } = mountPipelineView();

    await flushPromises();

    await expectRunnerCollapsed(wrapper);
  });

  it('starts collapsed when mounted with a stuck pipeline view', async () => {
    const { wrapper } = mountPipelineView((store) => {
      store.currentView = createView('stuck');
    });

    await flushPromises();

    await expectRunnerCollapsed(wrapper);
  });

  it('expands when a pipeline error appears after mount', async () => {
    const { wrapper } = mountPipelineView();

    await flushPromises();
    mcpState.error.value = 'live connection failure';
    await nextTick();

    await expectRunnerExpanded(wrapper);
  });

  it('collapses again when switching pipelines', async () => {
    const { store, wrapper } = mountPipelineView();

    await flushPromises();
    await runnerToggle(wrapper).trigger('click');
    await expectRunnerExpanded(wrapper);

    store.currentPipelineId = 'pipeline-2';
    await nextTick();

    await expectRunnerCollapsed(wrapper);
  });

  it('stays collapsed when switching into a stuck pipeline view', async () => {
    const { store, wrapper } = mountPipelineView((pipelineStore) => {
      pipelineStore.currentPipelineId = 'pipeline-a';
      pipelineStore.currentView = createView('completed', 'pipeline-a');
    });

    await flushPromises();
    await expectRunnerCollapsed(wrapper);

    store.currentPipelineId = 'pipeline-b';
    store.currentView = createView('stuck', 'pipeline-b');
    await settleRunnerWatchers();

    await expectRunnerCollapsed(wrapper);
  });

  it('expands when the same pipeline transitions to stuck', async () => {
    const { store, wrapper } = mountPipelineView((pipelineStore) => {
      pipelineStore.currentPipelineId = 'pipeline-a';
      pipelineStore.currentView = createView('completed', 'pipeline-a');
    });

    await flushPromises();
    await expectRunnerCollapsed(wrapper);

    store.currentView = createView('stuck', 'pipeline-a');
    await settleRunnerWatchers();

    await expectRunnerExpanded(wrapper);
  });

  it('expands when the same pipeline transitions to abandoned', async () => {
    const { store, wrapper } = mountPipelineView((pipelineStore) => {
      pipelineStore.currentPipelineId = 'pipeline-a';
      pipelineStore.currentView = createView('completed', 'pipeline-a');
    });

    await flushPromises();
    await expectRunnerCollapsed(wrapper);

    store.currentView = createView('abandoned', 'pipeline-a');
    await settleRunnerWatchers();

    await expectRunnerExpanded(wrapper);
  });

  it('does not re-expand when re-entering a stuck pipeline', async () => {
    const { store, wrapper } = mountPipelineView((pipelineStore) => {
      pipelineStore.currentPipelineId = 'pipeline-a';
      pipelineStore.currentView = createView('stuck', 'pipeline-a');
    });

    await flushPromises();
    await expectRunnerCollapsed(wrapper);

    store.currentPipelineId = 'pipeline-b';
    store.currentView = createView('completed', 'pipeline-b');
    await settleRunnerWatchers();
    await expectRunnerCollapsed(wrapper);

    store.currentPipelineId = 'pipeline-a';
    store.currentView = createView('stuck', 'pipeline-a');
    await settleRunnerWatchers();

    await expectRunnerCollapsed(wrapper);
  });
});
