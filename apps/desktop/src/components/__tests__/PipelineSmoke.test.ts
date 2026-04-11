// @vitest-environment jsdom
import { defineComponent, ref } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';

import PipelineComposer from '../PipelineComposer.vue';
import PipelineInspector from '../PipelineInspector.vue';
import PipelineRunner from '../PipelineRunner.vue';
import { isWorkflowNode } from '../../lib/pipeline-types';
import PipelineSidebar from '../PipelineSidebar.vue';
import PipelineView from '../PipelineView.vue';
import { useNodePaletteStore } from '../../stores/node-palette';
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

function findButton(wrapper: VueWrapper<any>, label: string) {
  return wrapper.findAll('button').find((button) => button.text() === label);
}

beforeEach(() => {
  vi.restoreAllMocks();
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

describe('pipeline smoke tests', () => {
  it('PipelineSidebar renders and enters design mode from + New', async () => {
    const { pinia, store } = createStore();
    store.designMode = false;

    const wrapper = mount(PipelineSidebar, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.get('h3').text()).toBe('Pipelines');
    const newButton = findButton(wrapper, '+ New');
    expect(newButton).toBeTruthy();
    expect(newButton!.text()).toBe('+ New');

    await newButton!.trigger('click');

    expect(store.designMode).toBe(true);
  });

  it('PipelineSidebar confirms before discarding a dirty draft on pipeline switch', async () => {
    const { pinia, store } = createStore();
    store.designMode = true;
    store.localDefinition = {
      nodes: [{ id: 'draft-node', workflowId: 'workflow.alpha' }],
      edges: [],
    };
    store.currentPipelineId = 'pipeline-current';
    store.pipelines = [
      { pipelineId: 'pipeline-current', status: 'paused', nodeCount: 1, completedCount: 0 },
      { pipelineId: 'pipeline-next', status: 'paused', nodeCount: 1, completedCount: 0 },
    ];

    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const loadPipelineSpy = vi.spyOn(store, 'loadPipeline').mockResolvedValue(true);

    const wrapper = mount(PipelineSidebar, {
      global: { plugins: [pinia] },
    });

    await wrapper.get('[data-pipeline-id="pipeline-next"]').trigger('click');

    expect(confirmSpy).toHaveBeenCalledWith('Discard unsaved pipeline draft?');
    expect(loadPipelineSpy).not.toHaveBeenCalled();
    expect(wrapper.emitted('pipeline-selected')).toBeUndefined();
  });

  it('PipelineSidebar emits only after a successful pipeline load', async () => {
    const { pinia, store } = createStore();
    store.designMode = false;
    store.currentPipelineId = 'pipeline-current';
    store.pipelines = [
      { pipelineId: 'pipeline-current', status: 'paused', nodeCount: 1, completedCount: 0 },
      { pipelineId: 'pipeline-next', status: 'paused', nodeCount: 1, completedCount: 0 },
    ];

    const loadPipelineSpy = vi.spyOn(store, 'loadPipeline')
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);

    const wrapper = mount(PipelineSidebar, {
      global: { plugins: [pinia] },
    });

    await wrapper.get('[data-pipeline-id="pipeline-next"]').trigger('click');
    expect(loadPipelineSpy).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('pipeline-selected')).toBeUndefined();

    await wrapper.get('[data-pipeline-id="pipeline-next"]').trigger('click');
    expect(loadPipelineSpy).toHaveBeenCalledTimes(2);
    expect(wrapper.emitted('pipeline-selected')).toEqual([['pipeline-next']]);
  });

  it('PipelineComposer adds a node through the design toolbar', async () => {
    const { pinia, store } = createStore();
    const nodePaletteStore = useNodePaletteStore();
    nodePaletteStore.setPanelWidth(432);

    const ComposerHarness = defineComponent({
      components: { PipelineComposer },
      setup() {
        const selectedNodeId = ref<string | null>(null);
        const selectedEdge = ref<{ source: string; target: string } | null>(null);
        return { store, selectedNodeId, selectedEdge };
      },
      template: `
        <PipelineComposer
          :design-mode="store.designMode"
          :local-definition="store.localDefinition"
          @update:local-definition="store.localDefinition = $event"
          @node-selected="selectedNodeId = $event"
          @edge-selected="selectedEdge = $event"
        />
      `,
    });

    const wrapper = mount(ComposerHarness, {
      global: {
        plugins: [pinia],
        stubs: vueFlowStubs,
      },
    });

    expect(wrapper.find('input[placeholder="Workflow ID"]').exists()).toBe(true);
    expect(wrapper.find('.composer-layout').attributes('style')).toContain('432px');
    expect(findButton(wrapper, 'Add Node')).toBeTruthy();

    await wrapper.get('input[placeholder="Workflow ID"]').setValue('workflow.alpha');
    await wrapper.get('form.composer-toolbar').trigger('submit');
    await flushPromises();

    expect(store.localDefinition.nodes).toHaveLength(1);
    expect(isWorkflowNode(store.localDefinition.nodes[0]!)).toBe(true);
    if (isWorkflowNode(store.localDefinition.nodes[0]!)) {
      expect(store.localDefinition.nodes[0].workflowId).toBe('workflow.alpha');
    }
  });

  it('PipelineRunner only shows Create when design mode has nodes to create', async () => {
    const first = createStore();
    first.store.localDefinition = {
      nodes: [{ id: 'node-1', workflowId: 'workflow.alpha' }],
      edges: [],
    };

    const visibleWrapper = mount(PipelineRunner, {
      global: { plugins: [first.pinia] },
    });

    expect(findButton(visibleWrapper, 'Create')).toBeTruthy();

    const second = createStore();
    second.store.localDefinition = { nodes: [], edges: [] };

    const hiddenWrapper = mount(PipelineRunner, {
      global: { plugins: [second.pinia] },
    });

    expect(findButton(hiddenWrapper, 'Create')).toBeFalsy();
  });

  it('PipelineInspector switches between design overview and node inspector', async () => {
    const overview = createStore();

    const overviewWrapper = mount(PipelineInspector, {
      props: {
        selectedNodeId: null,
        selectedEdge: null,
      },
      global: { plugins: [overview.pinia] },
    });

    expect(overviewWrapper.get('h2').text()).toBe('Pipeline Settings');

    const nodeCase = createStore();
    nodeCase.store.localDefinition = {
      nodes: [{ id: 'node-1', workflowId: 'workflow.alpha' }],
      edges: [],
    };

    const nodeWrapper = mount(PipelineInspector, {
      props: {
        selectedNodeId: 'node-1',
        selectedEdge: null,
      },
      global: { plugins: [nodeCase.pinia] },
    });

    expect(nodeWrapper.get('h2').text()).toBe('Node');
  });

  it('PipelineView mounts the sidebar, composer, inspector, and runner panels together', async () => {
    const { pinia, store } = createStore();
    store.localDefinition = {
      nodes: [{ id: 'node-1', workflowId: 'workflow.alpha' }],
      edges: [],
    };
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

    await flushPromises();

    expect(wrapper.findComponent(PipelineSidebar).exists()).toBe(true);
    expect(wrapper.findComponent(PipelineComposer).exists()).toBe(true);
    expect(wrapper.findComponent(PipelineInspector).exists()).toBe(true);
    expect(wrapper.findComponent(PipelineRunner).exists()).toBe(true);
  });
});
