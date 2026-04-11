// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

import ToolNodeFields from '../ToolNodeFields.vue';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import { useAgentCapabilitiesStore } from '../../stores/agent-capabilities';
import { useConfigStore } from '../../stores/config';
import { useMcpToolDiscoveryStore } from '../../stores/mcp-tool-discovery';
import { useNodePaletteStore } from '../../stores/node-palette';
import { usePipelineStore } from '../../stores/pipeline';
import type { McpServerDiscoveryResult } from '../../lib/types';

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

function fakeDiscoveryResults(): McpServerDiscoveryResult[] {
  return [
    {
      server: 'maenifold',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'maenifold.WriteMemory',
          description: 'Write <b>memory</b> safely',
          inputSchema: { type: 'object', properties: {} },
        },
        {
          name: 'maenifold.SearchMemories',
          description: 'Search memories',
          inputSchema: {
            type: 'object',
            required: ['query'],
            properties: { query: { type: 'string' } },
          },
        },
      ],
    },
    {
      server: 'filesystem',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'filesystem.ReadFile',
          description: 'Read a file',
          inputSchema: { type: 'object', properties: {} },
        },
      ],
    },
  ];
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
      offline: { command: 'offline' },
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
      ],
    })],
    ['offline', normalizeAgentProbeResult({}, { error: 'network error' })],
  ]);

  const discoveryStore = useMcpToolDiscoveryStore();
  discoveryStore.serverResults = fakeDiscoveryResults();
  discoveryStore.hasLoadedCatalog = true;

  const pipelineStore = usePipelineStore();
  pipelineStore.tools = [];

  const nodePaletteStore = useNodePaletteStore();

  return { pinia, nodePaletteStore };
}

beforeEach(() => {
  vi.restoreAllMocks();
  installMockLocalStorage();
});

describe('ToolNodeFields', () => {
  it('shows Quick Add from pinned and recent tools only when search is empty', async () => {
    const { pinia, nodePaletteStore } = createTestEnv();
    nodePaletteStore.togglePinnedTool('filesystem.ReadFile');
    nodePaletteStore.recordRecentTool('maenifold.SearchMemories');

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await flushPromises();

    expect(wrapper.text()).toContain('Quick Add');
    const options = wrapper.findAll('[role="option"]');
    expect(options).toHaveLength(2);
    expect(options[0]!.text()).toContain('ReadFile');
    expect(options[1]!.text()).toContain('SearchMemories');
  });

  it('shows a flat cross-server result list with result count and escaped highlighting when searching', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await wrapper.get('input[type="search"]').setValue('mem');
    await flushPromises();

    expect(wrapper.text()).toContain('2 results');
    expect(wrapper.findAll('[role="option"]')).toHaveLength(2);
    expect(wrapper.findAll('mark').map((node) => node.text().toLowerCase())).toContain('mem');
    expect(wrapper.html()).not.toContain('<b>memory</b>');
    expect(wrapper.html()).toContain('&lt;b&gt;');
  });

  it('does not block spacebar text entry in the search input', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    const searchInput = wrapper.get('input[type="search"]');
    const keydown = new KeyboardEvent('keydown', { key: ' ', bubbles: true, cancelable: true });

    expect(searchInput.element.dispatchEvent(keydown)).toBe(true);
    expect(keydown.defaultPrevented).toBe(false);
  });

  it('supports keyboard listbox navigation and submit without trapping focus', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
        canSubmit: true,
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await wrapper.get('input[type="search"]').setValue('read');
    await flushPromises();

    const listbox = wrapper.get('[role="listbox"]');
    await listbox.trigger('keydown', { key: 'ArrowDown' });
    await flushPromises();

    const emitted = wrapper.emitted('update:toolName');
    expect(emitted).toBeTruthy();
    expect(emitted![0]![0]).toBe('filesystem.ReadFile');

    await wrapper.setProps({ toolName: 'filesystem.ReadFile' });
    await listbox.trigger('keydown', { key: 'Enter' });
    await listbox.trigger('keydown', { key: ' ' });

    expect(listbox.attributes('aria-activedescendant')).toContain('filesystem-ReadFile');
    expect(listbox.attributes('tabindex')).toBe('0');
    expect(wrapper.emitted('submitRequest')).toHaveLength(2);
  });

  it('preserves persisted quick-add history until the catalog has really loaded', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    installMockLocalStorage();
    window.localStorage.setItem('pipeline-node-palette-v1', JSON.stringify({
      recentTools: ['filesystem.ReadFile'],
      pinnedTools: ['maenifold.SearchMemories'],
    }));

    const configStore = useConfigStore();
    configStore.config = {
      mcpServers: [],
      agents: {
        architect: { command: 'architect' },
      },
    };

    const capabilitiesStore = useAgentCapabilitiesStore();
    capabilitiesStore.probeResults = new Map();

    const discoveryStore = useMcpToolDiscoveryStore();
    discoveryStore.serverResults = [];
    discoveryStore.hasLoadedCatalog = false;

    const pipelineStore = usePipelineStore();
    pipelineStore.tools = [];

    const nodePaletteStore = useNodePaletteStore();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: false,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await flushPromises();

    expect(nodePaletteStore.pinnedTools).toEqual(['maenifold.SearchMemories']);
    expect(nodePaletteStore.recentTools).toEqual(['filesystem.ReadFile']);
    expect(wrapper.text()).toContain('Refresh to load tools from MCP servers.');

    discoveryStore.serverResults = fakeDiscoveryResults();
    discoveryStore.hasLoadedCatalog = true;
    await flushPromises();

    const optionTexts = wrapper.findAll('[role="option"]').map((option) => option.text());
    expect(optionTexts).toHaveLength(2);
    expect(optionTexts[0]).toContain('SearchMemories');
    expect(optionTexts[1]).toContain('ReadFile');
  });

  it('shows offline fallback for tool agent override when probe fails', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: 'filesystem.ReadFile',
        agentName: 'offline',
        agentConfig: {},
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await flushPromises();

    expect(wrapper.text()).toContain('Offline');
    expect(wrapper.text()).toContain('network error');
    expect(wrapper.find('[data-testid="tool-model-select"]').exists()).toBe(false);
  });
});
