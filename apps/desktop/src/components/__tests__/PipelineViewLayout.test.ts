// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';

import PipelineComposer from '../PipelineComposer.vue';
import PipelineIconRail from '../PipelineIconRail.vue';
import PipelineSidebar from '../PipelineSidebar.vue';
import PipelineView from '../PipelineView.vue';
import { usePipelineStore } from '../../stores/pipeline';

const vueFlowStubs = {
  VueFlow: {
    template: '<div data-testid="vue-flow"><slot /></div>',
  },
  Background: true,
  Controls: true,
  MiniMap: true,
};

function createStore() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = usePipelineStore();
  store.designMode = true;
  store.localDefinition = { nodes: [], edges: [] };
  store.currentPipelineId = null;
  store.currentView = null;
  store.pipelines = [];
  store.workflows = ['workflow.alpha'];
  store.maxConcurrency = 8;
  return { pinia, store };
}

function mockViewport(narrowViewport: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: narrowViewport,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

function mountPipelineView(
  options: {
    narrowViewport?: boolean;
    setupStore?: (store: ReturnType<typeof usePipelineStore>) => void;
  } = {},
) {
  mockViewport(options.narrowViewport ?? false);
  const { pinia, store } = createStore();
  options.setupStore?.(store);
  vi.spyOn(store, 'connect').mockResolvedValue();
  vi.spyOn(store, 'refreshList').mockResolvedValue();
  vi.spyOn(store, 'refreshWorkflows').mockResolvedValue();
  vi.spyOn(store, 'refreshTools').mockResolvedValue();

  const wrapper = mount(PipelineView, {
    global: {
      plugins: [pinia],
      stubs: vueFlowStubs,
    },
  });

  return { wrapper, store };
}

async function openFlyout(wrapper: VueWrapper<any>) {
  await wrapper.get('button[aria-label="Open pipelines flyout"]').trigger('click');
  await flushPromises();
}

beforeEach(() => {
  vi.restoreAllMocks();
  mockViewport(false);
});

describe('PipelineView layout behavior', () => {
  it('renders the desktop icon rail and only mounts the sidebar inside the flyout', async () => {
    const { wrapper } = mountPipelineView({
      setupStore(store) {
        store.pipelines = [
          { pipelineId: 'pipeline-current', status: 'paused', nodeCount: 1, completedCount: 0 },
        ];
      },
    });

    await flushPromises();

    expect(wrapper.findComponent(PipelineIconRail).exists()).toBe(true);
    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(false);
    expect(wrapper.find('#pipeline-flyout').exists()).toBe(false);

    await openFlyout(wrapper);

    expect(wrapper.find('#pipeline-flyout').exists()).toBe(true);
    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(true);

    await wrapper.get('button[aria-label="Close pipelines flyout"]').trigger('click');
    await flushPromises();

    expect(wrapper.find('#pipeline-flyout').exists()).toBe(false);
    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(false);
  });

  it('closes the flyout on Escape before clearing the inspector selection', async () => {
    const { wrapper } = mountPipelineView();

    await flushPromises();
    await openFlyout(wrapper);
    wrapper.findComponent(PipelineComposer).vm.$emit('nodeSelected', 'node-1');
    await flushPromises();

    expect(wrapper.find('.inspector-panel').classes()).toContain('is-visible');

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await flushPromises();

    expect(wrapper.find('#pipeline-flyout').exists()).toBe(false);
    expect(wrapper.find('.inspector-panel').classes()).toContain('is-visible');

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await flushPromises();

    expect(wrapper.find('.inspector-panel').classes()).not.toContain('is-visible');
  });

  it('keeps the flyout open for cancelled or failed switches and closes it after a successful switch', async () => {
    const { wrapper, store } = mountPipelineView({
      setupStore(currentStore) {
        currentStore.designMode = true;
        currentStore.localDefinition = {
          nodes: [{ id: 'draft-node', workflowId: 'workflow.alpha' }],
          edges: [],
        };
        currentStore.currentPipelineId = 'pipeline-current';
        currentStore.pipelines = [
          { pipelineId: 'pipeline-current', status: 'paused', nodeCount: 1, completedCount: 0 },
          { pipelineId: 'pipeline-next', status: 'paused', nodeCount: 1, completedCount: 0 },
        ];
      },
    });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const loadPipelineSpy = vi.spyOn(store, 'loadPipeline')
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);

    await flushPromises();
    await openFlyout(wrapper);

    await wrapper.get('[data-pipeline-id="pipeline-next"]').trigger('click');
    await flushPromises();

    expect(confirmSpy).toHaveBeenCalledWith('Discard unsaved pipeline draft?');
    expect(loadPipelineSpy).not.toHaveBeenCalled();
    expect(wrapper.find('#pipeline-flyout').exists()).toBe(true);

    confirmSpy.mockReturnValue(true);
    store.designMode = false;
    store.localDefinition = { nodes: [], edges: [] };

    await wrapper.get('[data-pipeline-id="pipeline-next"]').trigger('click');
    await flushPromises();

    expect(loadPipelineSpy).toHaveBeenCalledTimes(1);
    expect(wrapper.find('#pipeline-flyout').exists()).toBe(true);

    await wrapper.get('[data-pipeline-id="pipeline-next"]').trigger('click');
    await flushPromises();

    expect(loadPipelineSpy).toHaveBeenCalledTimes(2);
    expect(wrapper.find('#pipeline-flyout').exists()).toBe(false);
  });

  it('keeps mobile behavior by hiding the rail and rendering the existing sidebar layout', async () => {
    const { wrapper } = mountPipelineView({ narrowViewport: true });

    await flushPromises();

    expect(wrapper.findComponent(PipelineIconRail).exists()).toBe(false);
    expect(wrapper.find('button[aria-label="Open pipelines flyout"]').exists()).toBe(false);
    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(true);
    expect(wrapper.text()).toContain('Open Sidebar');
  });
});
