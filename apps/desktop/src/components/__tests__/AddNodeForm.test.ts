// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

import AddNodeForm from '../AddNodeForm.vue';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import { useAgentCapabilitiesStore } from '../../stores/agent-capabilities';
import { useConfigStore } from '../../stores/config';
import { useMcpToolDiscoveryStore } from '../../stores/mcp-tool-discovery';
import { useNodePaletteStore } from '../../stores/node-palette';
import { usePipelineStore } from '../../stores/pipeline';

function installMockLocalStorage() {
  const storage = new Map<string, string>();
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
      clear: () => storage.clear(),
    },
  });
}

function createTestEnv() {
  const pinia = createPinia();
  setActivePinia(pinia);
  installMockLocalStorage();
  window.localStorage.clear();

  const configStore = useConfigStore();
  configStore.config = {
    mcpServers: [],
    agents: {
      architect: { command: 'architect' },
    },
  };

  const capabilitiesStore = useAgentCapabilitiesStore();
  capabilitiesStore.probeResults = new Map([
    ['architect', normalizeAgentProbeResult({
      configOptions: [
        {
          id: 'mode',
          name: 'Mode',
          category: 'mode',
          currentValue: 'prompt',
          options: [
            { value: 'prompt', name: 'Prompt' },
            { value: 'workflow', name: 'Workflow' },
          ],
        },
        {
          id: 'model',
          name: 'Model',
          category: 'model',
          currentValue: 'gpt-4.1',
          options: [
            { value: 'gpt-4.1', name: 'GPT-4.1' },
            { value: 'gpt-5-mini', name: 'GPT-5 mini' },
          ],
        },
        {
          id: 'thought_level',
          name: 'Thought Level',
          category: 'thought_level',
          currentValue: 'medium',
          options: [
            { value: 'low', name: 'Low' },
            { value: 'medium', name: 'Medium' },
            { value: 'high', name: 'High' },
          ],
        },
        {
          id: 'temperature',
          name: 'Temperature',
          currentValue: 0,
          options: [
            { value: '0', name: '0' },
            { value: '1', name: '1' },
          ],
        },
      ],
    })],
  ]);

  const discoveryStore = useMcpToolDiscoveryStore();
  discoveryStore.serverResults = [
    {
      server: 'maenifold',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'maenifold.SearchMemories',
          description: 'Search memories',
          inputSchema: { type: 'object', properties: {} },
        },
      ],
    },
  ];

  const pipelineStore = usePipelineStore();
  pipelineStore.tools = [];

  const nodePaletteStore = useNodePaletteStore();

  return { pinia, nodePaletteStore };
}

beforeEach(() => {
  vi.restoreAllMocks();
  installMockLocalStorage();
});

describe('AddNodeForm', () => {
  it('submits prompt node agent overrides via PipelineNodeAgentConfig', async () => {
    const { pinia } = createTestEnv();
    const submitNode = vi.fn();

    const wrapper = mount(AddNodeForm, {
      props: {
        workflows: ['workflow.alpha'],
        hasSelection: false,
        submitNode,
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await wrapper.get('select[aria-label="Node type"]').setValue('prompt');
    await wrapper.get('[data-testid="prompt-agent-select"]').setValue('architect');
    await flushPromises();

    await wrapper.get('[data-testid="prompt-mode-badge-workflow"]').trigger('click');
    await wrapper.get('[data-testid="prompt-model-select"]').setValue('gpt-5-mini');
    await wrapper.get('[data-testid="prompt-thought-level-select"]').setValue('high');
    await wrapper.get('[data-testid="prompt-config-temperature"]').setValue('1');
    await wrapper.get('textarea[placeholder="Required prompt template"]').setValue('Investigate the failure');
    await wrapper.get('form.composer-toolbar').trigger('submit');

    expect(submitNode).toHaveBeenCalledWith('prompt', {
      agent: 'architect',
      prompt: 'Investigate the failure',
      agentConfig: {
        mode: 'workflow',
        model: 'gpt-5-mini',
        thoughtLevel: 'high',
        configOptions: {
          temperature: 1,
        },
      },
    });
  });

  it('keeps tool palette state and selection across repeated adds', async () => {
    const { pinia, nodePaletteStore } = createTestEnv();
    const submitNode = vi.fn();

    const wrapper = mount(AddNodeForm, {
      props: {
        workflows: ['workflow.alpha'],
        hasSelection: false,
        submitNode,
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await wrapper.get('select[aria-label="Node type"]').setValue('tool');
    await wrapper.get('input[type="search"]').setValue('search');
    await flushPromises();

    await wrapper.get('[role="option"]').trigger('click');
    await flushPromises();

    expect(nodePaletteStore.searchQuery).toBe('search');

    await wrapper.get('form.composer-toolbar').trigger('submit');

    expect(submitNode).toHaveBeenCalledWith('tool', {
      toolName: 'maenifold.SearchMemories',
    });
    expect(nodePaletteStore.searchQuery).toBe('search');
    expect(nodePaletteStore.selectedToolName).toBe('maenifold.SearchMemories');
    expect(nodePaletteStore.recentTools).toContain('maenifold.SearchMemories');
    expect((wrapper.get('input[type="search"]').element as HTMLInputElement).value).toBe('search');
  });
});
