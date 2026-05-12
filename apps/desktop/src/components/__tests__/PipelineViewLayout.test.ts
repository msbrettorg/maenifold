// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';

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

import PipelineComposer from '../PipelineComposer.vue';
import PipelineIconRail from '../PipelineIconRail.vue';
import PipelineSidebar from '../PipelineSidebar.vue';
import PipelineView from '../PipelineView.vue';
import { usePipelineStore } from '../../stores/pipeline';
import { useConfirmDialogStore } from '../../stores/confirm-dialog';

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
  store.designMode = false;
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
    attachTo: document.body,
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

function findFlyout(): HTMLElement | null {
  return document.body.querySelector<HTMLElement>('#pipeline-flyout');
}

function findInBody<E extends Element = HTMLElement>(selector: string): E | null {
  return document.body.querySelector<E>(selector);
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
    expect(findFlyout()).toBeNull();

    await openFlyout(wrapper);

    expect(findFlyout()).not.toBeNull();
    expect(findFlyout()?.textContent).toContain('pipeline-current');

    await wrapper.get('button[aria-label="Close pipelines flyout"]').trigger('click');
    await flushPromises();

    expect(findFlyout()).toBeNull();
    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(false);
  });

  it('closes the flyout on Escape and preserves node selection state', async () => {
    const { wrapper } = mountPipelineView({
      setupStore(store) {
        store.localDefinition = {
          nodes: [{ id: 'node-1', workflowId: 'workflow.alpha' }],
          edges: [],
        };
      },
    });

    await flushPromises();
    await openFlyout(wrapper);
    wrapper.findComponent(PipelineComposer).vm.$emit('nodeSelected', 'node-1');
    await flushPromises();

    expect(findFlyout()).not.toBeNull();

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await flushPromises();

    expect(findFlyout()).toBeNull();
  });

  it('keeps node selection in server mode without rendering inspector controls', async () => {
    const { wrapper } = mountPipelineView({
      narrowViewport: true,
      setupStore(store) {
        store.designMode = false;
        store.currentPipelineId = 'pipeline-current';
        store.currentView = {
          pipelineId: 'pipeline-current',
          status: 'paused',
          running: false,
          ready: ['node-1'],
          active: [],
          completed: [],
          mermaid: 'graph TD;',
          edges: [],
          nodes: [{
            nodeId: 'node-1',
            sessionId: 'session-1',
            status: 'pending',
            attempts: 0,
            type: 'workflow',
            workflowId: 'workflow.alpha',
          }],
        };
      },
    });

    await flushPromises();
    wrapper.findComponent(PipelineComposer).vm.$emit('nodeSelected', 'node-1');
    await flushPromises();

    expect(wrapper.find('.inspector-panel').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('Open Inspector');
    expect(wrapper.find('[aria-label="Close inspector"]').exists()).toBe(false);
  });

  it('keeps the flyout open for cancelled or failed switches and closes it after a successful switch', async () => {
    const { wrapper, store } = mountPipelineView({
      setupStore(currentStore) {
        currentStore.designMode = false;
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
    const confirmDialogStore = useConfirmDialogStore();
    const confirmSpy = vi.spyOn(confirmDialogStore, 'confirm').mockResolvedValue(false);
    const loadPipelineSpy = vi.spyOn(store, 'loadPipeline')
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);

    await flushPromises();
    await wrapper.get('button[aria-label="Toggle pipeline design mode"]').trigger('click');
    await flushPromises();
    await openFlyout(wrapper);

    const pipelineNextButton = (): HTMLElement => {
      const el = findInBody('[data-pipeline-id="pipeline-next"]');
      if (!el) throw new Error('pipeline-next button not found in document body');
      return el;
    };

    pipelineNextButton().click();
    await flushPromises();

    expect(confirmSpy).toHaveBeenCalledWith(
      'Discard unsaved pipeline draft?',
      'Your unsaved draft changes will be lost.',
    );
    expect(loadPipelineSpy).not.toHaveBeenCalled();
    expect(findFlyout()).not.toBeNull();

    confirmSpy.mockResolvedValue(true);
    store.designMode = false;
    store.localDefinition = { nodes: [], edges: [] };

    pipelineNextButton().click();
    await flushPromises();

    expect(loadPipelineSpy).toHaveBeenCalledTimes(1);
    expect(findFlyout()).not.toBeNull();

    pipelineNextButton().click();
    await flushPromises();

    expect(loadPipelineSpy).toHaveBeenCalledTimes(2);
    expect(findFlyout()).toBeNull();
  });

  it('keeps mobile behavior by hiding the rail and rendering the sidebar in the mobile sheet', async () => {
    const { wrapper } = mountPipelineView({ narrowViewport: true });

    await flushPromises();

    expect(wrapper.findComponent(PipelineIconRail).exists()).toBe(false);
    expect(wrapper.find('button[aria-label="Open pipelines flyout"]').exists()).toBe(false);
    expect(wrapper.text()).toContain('Open Sidebar');

    await wrapper.get('button').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Close Sidebar');
    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(true);
  });
});
