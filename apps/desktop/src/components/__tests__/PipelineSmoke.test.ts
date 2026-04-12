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
import { useConfigStore } from '../../stores/config';
import { useNodePaletteStore } from '../../stores/node-palette';
import { usePipelineStore } from '../../stores/pipeline';

const vueFlowStubs = {
  VueFlow: {
    template: '<div class="vue-flow" data-testid="vue-flow"><slot /></div>',
  },
  Background: true,
  Controls: true,
  MiniMap: true,
};

function createStore() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const configStore = useConfigStore();
  configStore.config = {
    mcpServers: [],
    agents: {
      architect: { command: 'architect' },
    },
  };
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

function installMatchMedia(initialMatches = false) {
  let matches = initialMatches;
  const listeners = new Set<(event: MediaQueryListEvent) => void>();

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      get matches() { return matches; },
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn((event: string, listener: (event: MediaQueryListEvent) => void) => {
        if (event === 'change') listeners.add(listener);
      }),
      removeEventListener: vi.fn((event: string, listener: (event: MediaQueryListEvent) => void) => {
        if (event === 'change') listeners.delete(listener);
      }),
      dispatchEvent: vi.fn(),
    })),
  });

  return {
    setMatches(nextMatches: boolean) {
      matches = nextMatches;
      const event = { matches, media: '(max-width: 900px)' } as MediaQueryListEvent;
      listeners.forEach((listener) => listener(event));
    },
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  Object.defineProperty(window, 'DOMMatrixReadOnly', {
    writable: true,
    configurable: true,
    value: class {
      a = 1;
      m41 = 0;
      m42 = 0;
    },
  });
  installMatchMedia();
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

  it('PipelineComposer keeps the floating toolbar visible in design mode, starts with the palette closed, and toggles the palette on desktop', async () => {
    const { pinia, store } = createStore();
    store.localDefinition = {
      nodes: [{ id: 'seed-node', workflowId: 'workflow.alpha' }],
      edges: [],
    };
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

    const paletteToggle = findButton(wrapper, '+ Palette');
    expect(findButton(wrapper, '↔ Layout LR')).toBeTruthy();
    expect(findButton(wrapper, '↕ Layout TB')).toBeTruthy();
    expect(findButton(wrapper, '⊞ Fit View')).toBeTruthy();
    expect(paletteToggle).toBeTruthy();
    expect(wrapper.find('input[placeholder="Workflow ID"]').exists()).toBe(false);
    expect(wrapper.find('.composer-layout').attributes('style')).toContain('minmax(0, 1fr)');

    await paletteToggle!.trigger('click');
    expect(wrapper.find('input[placeholder="Workflow ID"]').exists()).toBe(true);
    expect(wrapper.find('.composer-layout').attributes('style')).toContain('432px');
    expect(findButton(wrapper, 'Add Node')).toBeTruthy();

    await wrapper.get('input[placeholder="Workflow ID"]').setValue('workflow.alpha');
    await wrapper.get('form.composer-toolbar').trigger('submit');
    await flushPromises();

    expect(store.localDefinition.nodes).toHaveLength(2);
    const createdNode = store.localDefinition.nodes[1]!;
    expect(isWorkflowNode(createdNode)).toBe(true);
    if (isWorkflowNode(createdNode)) {
      expect(createdNode.workflowId).toBe('workflow.alpha');
    }

    await findButton(wrapper, '✕ Palette')!.trigger('click');
    expect(wrapper.find('input[placeholder="Workflow ID"]').exists()).toBe(false);
  });

  it('PipelineComposer keeps the toolbar and context-menu-first entry path available in the empty gallery state', async () => {
    const { pinia, store } = createStore();

    const wrapper = mount(PipelineComposer, {
      props: {
        designMode: store.designMode,
        localDefinition: store.localDefinition,
      },
      global: {
        plugins: [pinia],
        stubs: vueFlowStubs,
      },
    });

    expect(wrapper.find('.composer-empty-state').exists()).toBe(true);
    expect(wrapper.text()).toContain('Add nodes using the form or right-click the canvas');
    expect(findButton(wrapper, '+ Palette')).toBeTruthy();

    await wrapper.get('.composer-canvas').trigger('contextmenu', { clientX: 160, clientY: 120 });
    await flushPromises();

    expect(wrapper.find('.composer-context-menu').exists()).toBe(true);
    expect(wrapper.text()).toContain('Ask an agent…');
    expect(wrapper.text()).toContain('Run a workflow…');
  });

  it('PipelineComposer forces the palette closed on narrow viewports', async () => {
    const viewport = installMatchMedia(false);
    const { pinia, store } = createStore();

    const wrapper = mount(PipelineComposer, {
      props: {
        designMode: store.designMode,
        localDefinition: {
          nodes: [{ id: 'seed-node', workflowId: 'workflow.alpha' }],
          edges: [],
        },
      },
      global: {
        plugins: [pinia],
        stubs: vueFlowStubs,
      },
    });

    await findButton(wrapper, '+ Palette')!.trigger('click');
    expect(wrapper.find('input[placeholder="Workflow ID"]').exists()).toBe(true);

    viewport.setMatches(true);
    await flushPromises();

    const paletteToggle = wrapper.get('button.composer-palette-toggle');
    expect(wrapper.find('input[placeholder="Workflow ID"]').exists()).toBe(false);
    expect(paletteToggle.attributes('aria-pressed')).toBe('false');
  });

  it('PipelineRunner stays read-only and keeps only status/log output', async () => {
    const first = createStore();
    first.store.localDefinition = {
      nodes: [{ id: 'node-1', workflowId: 'workflow.alpha' }],
      edges: [],
    };
    first.store.eventLog = [{
      nodeId: 'node-1',
      from: 'pending',
      to: 'completed',
      timestamp: Date.parse('2026-04-11T12:00:00Z'),
    }];

    const visibleWrapper = mount(PipelineRunner, {
      global: { plugins: [first.pinia] },
    });

    expect(findButton(visibleWrapper, 'Create')).toBeFalsy();
    expect(findButton(visibleWrapper, 'Run')).toBeFalsy();
    expect(findButton(visibleWrapper, 'Pause')).toBeFalsy();
    expect(findButton(visibleWrapper, 'Cancel')).toBeFalsy();
    expect(visibleWrapper.text()).toContain('Design mode');
    expect(visibleWrapper.text()).toContain('node-1: pending → completed');
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

    expect(nodeWrapper.get('h2').text()).toBe('node-1');
  });

});
