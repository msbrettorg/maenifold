// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import { ref } from 'vue';

import ToolPaletteFields from '../ToolPaletteFields.vue';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import { useAgentCapabilitiesStore } from '../../stores/agent-capabilities';
import { useConfigStore } from '../../stores/config';
import { useMcpToolDiscoveryStore } from '../../stores/mcp-tool-discovery';
import { useNodePaletteStore } from '../../stores/node-palette';
import { usePipelineStore } from '../../stores/pipeline';
import type { McpServerDiscoveryResult } from '../../lib/types';

const { callTool, connect, connected, discoveryResultsQueue } = vi.hoisted(() => {
  const connected = { value: false, __v_isRef: true };
  const discoveryResultsQueue: unknown[][] = [];
  return {
    callTool: vi.fn(async (toolName: string) => (
      toolName === 'DiscoverMcpTools'
        ? (discoveryResultsQueue.shift() ?? [])
        : {}
    )),
    connect: vi.fn(async () => { connected.value = true; }),
    connected,
    discoveryResultsQueue,
  };
});

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected,
    reconnecting: ref(false),
    error: ref<string | null>(null),
    connect,
    callTool,
    listTools: vi.fn(async () => []),
    diffViews: vi.fn(() => []),
    startPolling: vi.fn(),
  }),
}));

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => ({
    connected,
    reconnecting: ref(false),
    error: ref<string | null>(null),
    connect,
    disconnect: vi.fn(async () => { connected.value = false; }),
    callTool,
  }),
}));

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

function healthyDiscoveryResults(): McpServerDiscoveryResult[] {
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
      server: 'msx-mcp',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'msx-mcp.get_account_overview',
          description: 'Get account overview',
          inputSchema: { type: 'object', properties: {} },
        },
        {
          name: 'msx-mcp.get_my_deals',
          description: 'Get my deals',
          inputSchema: { type: 'object', properties: {} },
        },
      ],
    },
    {
      server: 'workiq',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'workiq.ask_work_iq',
          description: 'Ask work intelligence',
          inputSchema: { type: 'object', properties: {} },
        },
      ],
    },
    {
      server: 'ranking',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'ranking.scope',
          description: 'Exact scope match',
          inputSchema: { type: 'object', properties: {} },
        },
        {
          name: 'ranking.ScopeStart',
          description: 'Starts with scope',
          inputSchema: { type: 'object', properties: {} },
        },
        {
          name: 'ranking.alpha_scope',
          description: 'Contains scope as a later word',
          inputSchema: { type: 'object', properties: {} },
        },
        {
          name: 'ranking.microscope',
          description: 'Contains scope as substring',
          inputSchema: { type: 'object', properties: {} },
        },
      ],
    },
  ];
}

function mockDiscoveryFetch(...results: McpServerDiscoveryResult[][]) {
  discoveryResultsQueue.splice(0, discoveryResultsQueue.length, ...results);
  vi.stubGlobal(
    'fetch',
    vi.fn().mockImplementation(() => {
      const next = results.shift() ?? healthyDiscoveryResults();
      return Promise.resolve({
        ok: true,
        json: async () => next,
      });
    }),
  );
}

function firstRunErrorDiscoveryResults(): McpServerDiscoveryResult[] {
  const healthy = healthyDiscoveryResults();
  return [
    healthy[0]!,
    {
      server: 'msx-mcp',
      type: 'stdio',
      status: 'error',
      error: 'login required',
      tools: [],
    },
    {
      server: 'workiq',
      type: 'stdio',
      status: 'error',
      error: 'EULA acceptance required',
      tools: [],
    },
    {
      server: 'broken',
      type: 'stdio',
      status: 'error',
      error: 'socket refused',
      tools: [],
    },
    healthy[3]!,
  ];
}

async function createTestEnv(results: McpServerDiscoveryResult[] = healthyDiscoveryResults()) {
  const pinia = createPinia();
  setActivePinia(pinia);
  installMockLocalStorage();
  window.localStorage.clear();

  useConfigStore().config = { mcpServers: [], agents: { architect: { command: 'architect' } } };
  useAgentCapabilitiesStore().probeResults = new Map([
    ['architect', normalizeAgentProbeResult({ configOptions: [] })],
  ]);

  const discoveryStore = useMcpToolDiscoveryStore();
  discoveryStore.serverResults = results;
  discoveryStore.hasLoadedCatalog = true;

  usePipelineStore().tools = [];
  const nodePaletteStore = useNodePaletteStore();

  return { pinia, discoveryStore, nodePaletteStore };
}

async function mountPalette(results: McpServerDiscoveryResult[] = healthyDiscoveryResults(), canSubmit = false) {
  const env = await createTestEnv(results);
  const wrapper = mount(ToolPaletteFields, {
    attachTo: document.body,
    props: {
      active: true,
      toolName: '',
      agentName: '',
      agentConfig: {},
      toolArgs: {},
      canSubmit,
    },
    global: {
      plugins: [env.pinia],
      stubs: { SchemaForm: true },
    },
  });

  return { wrapper, ...env };
}

beforeEach(() => {
  vi.restoreAllMocks();
  connected.value = false;
  connect.mockClear();
  callTool.mockClear();
  mockDiscoveryFetch(healthyDiscoveryResults());
});

afterEach(() => {
  document.body.innerHTML = '';
});

describe('ToolPaletteFields', () => {
  it('surfaces first-run auth-required servers even when discovery returns zero tools', async () => {
    const { wrapper } = await mountPalette(firstRunErrorDiscoveryResults());

    await wrapper.get('input[type="search"]').setValue('msx');
    await flushPromises();
    await wrapper.get('[role="option"]').trigger('click');

    expect(wrapper.text()).toContain('Sign in to MSX');
    expect(wrapper.text()).toContain('Setup Required');
    expect(wrapper.text()).toContain('login required');
    expect(wrapper.emitted('update:toolName')).toBeUndefined();
  });

  it('runs setup from a first-run auth card and refreshes the real tools after auth succeeds', async () => {
    const { wrapper, discoveryStore } = await mountPalette(firstRunErrorDiscoveryResults());
    mockDiscoveryFetch(healthyDiscoveryResults());

    await wrapper.get('input[type="search"]').setValue('msx');
    await flushPromises();
    await wrapper.get('[role="option"]').trigger('click');
    (document.body.querySelector('[data-testid="tool-auth-setup-confirm"]') as HTMLButtonElement).click();
    await flushPromises();

    expect(connect).toHaveBeenCalled();
    expect(callTool).toHaveBeenCalledWith('msx-mcp.msx_login', {});
    expect(discoveryStore.serverHealth['msx-mcp']).toBe('ready');
    expect(wrapper.find('.auth-setup-dialog').exists()).toBe(false);
    expect(wrapper.text()).toContain('get_account_overview');
  });

  it('ranks real search results and surfaces account/email synonym matches on the shipped palette', async () => {
    const { wrapper } = await mountPalette();

    await wrapper.get('input[type="search"]').setValue('scope');
    await flushPromises();

    expect(wrapper.findAll('[role="option"]').map((option) => option.text())).toEqual([
      expect.stringContaining('scope'),
      expect.stringContaining('ScopeStart'),
      expect.stringContaining('alpha_scope'),
      expect.stringContaining('microscope'),
    ]);

    await wrapper.get('input[type="search"]').setValue('mem');
    await flushPromises();
    expect(wrapper.findAll('mark').map((node) => node.text().toLowerCase())).toContain('mem');
    expect(wrapper.html()).not.toContain('<b>memory</b>');
    expect(wrapper.html()).toContain('&lt;b&gt;');

    await wrapper.get('input[type="search"]').setValue('account');
    await flushPromises();

    const accountOptions = wrapper.findAll('[role="option"]').map((option) => option.text());
    expect(accountOptions[0]).toContain('get_account_overview');
    expect(accountOptions[1]).toContain('get_my_deals');

    await wrapper.get('input[type="search"]').setValue('account data');
    await flushPromises();

    const accountDataOptions = wrapper.findAll('[role="option"]').map((option) => option.text());
    expect(accountDataOptions[0]).toContain('get_account_overview');
    expect(accountDataOptions[1]).toContain('get_my_deals');

    await wrapper.get('input[type="search"]').setValue('email');
    await flushPromises();

    const emailOptions = wrapper.findAll('[role="option"]').map((option) => option.text());
    expect(emailOptions).toHaveLength(1);
    expect(emailOptions[0]).toContain('ask_work_iq');
    expect(emailOptions[0]).toContain('workiq');
  });

  it('keeps browse mode in a keyboard-accessible listbox while grouping by intent category', async () => {
    const { wrapper, nodePaletteStore } = await mountPalette(healthyDiscoveryResults(), true);
    nodePaletteStore.togglePinnedTool('workiq.ask_work_iq');
    nodePaletteStore.recordRecentTool('maenifold.WriteMemory');

    await flushPromises();

    expect(wrapper.text()).toContain('Browse by intent');
    expect(wrapper.text()).toContain('Quick Add');
    expect(wrapper.text()).toContain('Memory & Knowledge');
    expect(wrapper.text()).toContain('Accounts & CRM');
    expect(wrapper.text()).toContain('Workplace Intelligence');
    expect(wrapper.text()).not.toContain('Documentation');
    const quickAdd = wrapper.get('section[aria-label="Quick Add"]');
    expect(quickAdd.text()).toContain('ask_work_iq');
    expect(quickAdd.text()).toContain('WriteMemory');

    const listbox = wrapper.get('[role="listbox"]');
    expect(listbox.attributes('aria-label')).toBe('Browse by intent tools');
    expect(listbox.attributes('aria-activedescendant')).toContain('maenifold-SearchMemories');

    await listbox.trigger('keydown', { key: 'ArrowDown' });
    await flushPromises();
    expect(listbox.attributes('aria-activedescendant')).toContain('maenifold-WriteMemory');

    await listbox.trigger('keydown', { key: 'Enter' });
    await flushPromises();
    expect(wrapper.emitted('update:toolName')?.[0]?.[0]).toBe('maenifold.WriteMemory');

    await wrapper.setProps({ toolName: 'maenifold.WriteMemory' });
    await listbox.trigger('keydown', { key: ' ' });
    expect(wrapper.emitted('submitRequest')).toHaveLength(1);
    expect(wrapper.findAll('[role="option"]').some((node) => node.text().includes('workiq'))).toBe(true);
  });

  it('blocks cached auth-error tools into auth setup path even when stale cache exists', async () => {
    const env = await createTestEnv();
    mockDiscoveryFetch(healthyDiscoveryResults(), firstRunErrorDiscoveryResults());
    await env.discoveryStore.refreshTools();
    await env.discoveryStore.refreshTools();

    const wrapper = mount(ToolPaletteFields, {
      attachTo: document.body,
      props: {
        active: true,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
        canSubmit: false,
      },
      global: {
        plugins: [env.pinia],
        stubs: { SchemaForm: true },
      },
    });

    await wrapper.get('input[type="search"]').setValue('account');
    await flushPromises();
    await wrapper.get('[role="option"]').trigger('click');

    expect(wrapper.text()).toContain('Setup Required');
    expect(document.body.textContent).toContain('Sign in to MSX');
    expect(wrapper.emitted('update:toolName')).toBeUndefined();
  });

  it('keeps stale cached tools selectable for non-auth server errors', async () => {
    const env = await createTestEnv();
    const healthy = healthyDiscoveryResults();
    mockDiscoveryFetch(
      healthy,
      [
        healthy[0]!,
        healthy[1]!,
        healthy[2]!,
        { server: 'ranking', type: 'stdio', status: 'error', error: 'connection timed out', tools: [] },
      ],
    );
    await env.discoveryStore.refreshTools();
    await env.discoveryStore.refreshTools();

    const wrapper = mount(ToolPaletteFields, {
      attachTo: document.body,
      props: {
        active: true,
        toolName: '',
        agentName: '',
        agentConfig: {},
        toolArgs: {},
        canSubmit: false,
      },
      global: {
        plugins: [env.pinia],
        stubs: { SchemaForm: true },
      },
    });

    await wrapper.get('input[type="search"]').setValue('scope');
    await flushPromises();
    await wrapper.get('[role="option"]').trigger('click');

    expect(wrapper.text()).toContain('Stale');
    expect(wrapper.emitted('update:toolName')?.[0]?.[0]).toBe('ranking.scope');
  });

  it('shows unavailable no-cache servers on first discovery', async () => {
    const { wrapper } = await mountPalette(firstRunErrorDiscoveryResults());

    await wrapper.get('input[type="search"]').setValue('broken');
    await flushPromises();

    expect(wrapper.text()).toContain('broken unavailable');
    expect(wrapper.text()).toContain('Unavailable');
    expect(wrapper.text()).toContain('socket refused');
  });
});
