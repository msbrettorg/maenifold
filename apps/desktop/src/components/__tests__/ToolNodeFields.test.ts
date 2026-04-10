// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import ToolNodeFields from '../ToolNodeFields.vue';
import { useMcpToolDiscoveryStore } from '../../stores/mcp-tool-discovery';
import { usePipelineStore } from '../../stores/pipeline';
import type { McpServerDiscoveryResult } from '../../lib/types';

/**
 * Regression test for T-123.3 — double-prefix bug.
 *
 * The backend (mcp_discovery.rs) already returns tool names as
 * "server.toolName". The picker must NOT re-prefix them, otherwise
 * selection emits "server.server.toolName".
 */

/** Backend-shaped discovery result — names are already namespaced. */
function fakeDiscoveryResults(): McpServerDiscoveryResult[] {
  return [
    {
      server: 'maenifold',
      type: 'stdio',
      status: 'ok',
      tools: [
        {
          name: 'maenifold.WriteMemory',
          description: 'Write a memory',
          inputSchema: { type: 'object', properties: {} },
        },
        {
          name: 'maenifold.SearchMemories',
          description: 'Search memories',
          inputSchema: { type: 'object', properties: {} },
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

  const discoveryStore = useMcpToolDiscoveryStore();
  discoveryStore.serverResults = fakeDiscoveryResults();

  const pipelineStore = usePipelineStore();
  pipelineStore.tools = [];

  return { pinia, discoveryStore, pipelineStore };
}

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('ToolNodeFields — tool name namespacing', () => {
  it('emits single-prefixed "server.toolName" on picker click (no double-prefix)', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: '',
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await flushPromises();

    const toolButtons = wrapper.findAll('.tool-picker__tool');
    expect(toolButtons.length).toBeGreaterThanOrEqual(3);

    // Click the first tool (maenifold.SearchMemories or maenifold.WriteMemory — sorted)
    await toolButtons[0]!.trigger('click');

    const emitted = wrapper.emitted('update:toolName');
    expect(emitted).toBeTruthy();
    expect(emitted![0]![0]).toBe('maenifold.SearchMemories');

    // Verify it does NOT double-prefix
    expect(emitted![0]![0]).not.toMatch(/^maenifold\.maenifold\./);
  });

  it('marks a tool as selected when toolName prop matches backend name', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: 'filesystem.ReadFile',
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await flushPromises();

    const selected = wrapper.findAll('.tool-picker__tool.is-selected');
    expect(selected).toHaveLength(1);
    expect(selected[0]!.text()).toContain('ReadFile');
  });

  it('displays bare tool name in button labels (not the namespaced form)', async () => {
    const { pinia } = createTestEnv();

    const wrapper = mount(ToolNodeFields, {
      props: {
        active: true,
        toolName: '',
        toolArgs: {},
      },
      global: {
        plugins: [pinia],
        stubs: { SchemaForm: true },
      },
    });

    await flushPromises();

    const labels = wrapper.findAll('.tool-picker__tool strong').map((el) => el.text());
    // Labels should be bare names, not "server.toolName"
    expect(labels).toContain('SearchMemories');
    expect(labels).toContain('WriteMemory');
    expect(labels).toContain('ReadFile');
    // Must NOT contain the double-prefixed form
    expect(labels).not.toContain('maenifold.SearchMemories');
    expect(labels).not.toContain('maenifold.WriteMemory');
  });
});
