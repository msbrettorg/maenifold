// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import McpPromptPicker from '../McpPromptPicker.vue';
import type { AvailableCommand } from '../../lib/types';

const mocks = vi.hoisted(() => ({
  connect: vi.fn(async () => undefined),
  connected: { value: true },
  getPrompt: vi.fn(async () => ({ messages: [{ role: 'user', content: { type: 'text', text: 'Resolved prompt' } }] })),
}));

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    connect: mocks.connect,
    connected: mocks.connected,
    getPrompt: mocks.getPrompt,
  }),
}));

vi.mock('../../composables/useMcpCompletion', () => ({
  useMcpCompletion: () => ({
    supported: { value: false },
    complete: vi.fn(),
  }),
}));

const command: AvailableCommand = {
  name: 'mcp.default.summarize',
  description: 'Summarize',
  _meta: {
    group: 'MCP prompts / default',
    'maenifold.mcpPrompt': {
      kind: 'mcp-prompt',
      serverName: 'default',
      promptName: 'summarize',
      arguments: [],
    },
  },
};

describe('McpPromptPicker', () => {
  it('resolves prompt content through getPrompt and emits the messages', async () => {
    const wrapper = mount(McpPromptPicker, {
      props: {
        command,
        open: true,
      },
      global: {
        stubs: {
          Dialog: { template: '<div><slot /></div>' },
          DialogContent: { template: '<div><slot /></div>' },
          DialogHeader: { template: '<div><slot /></div>' },
          DialogTitle: { template: '<div><slot /></div>' },
          DialogFooter: { template: '<div><slot /></div>' },
          Button: { template: '<button @click="$emit(\'click\')"><slot /></button>' },
          SchemaFormRenderer: { name: 'SchemaFormRenderer', template: '<div />' },
        },
      },
    });

    await wrapper.findAll('button')[1]?.trigger('click');

    expect(mocks.getPrompt).toHaveBeenCalledWith('summarize', {});
    expect(wrapper.emitted('resolved')?.[0]?.[0]).toEqual([
      { role: 'user', content: { type: 'text', text: 'Resolved prompt' } },
    ]);
  });
});
