// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import McpPromptPicker from '../McpPromptPicker.vue';

const mocks = vi.hoisted(() => ({
  getPrompt: vi.fn(),
  connect: vi.fn(async () => {}),
  connected: { value: true, __v_isRef: true },
  complete: vi.fn(),
}));

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    getPrompt: mocks.getPrompt,
    connect: mocks.connect,
    connected: mocks.connected,
  }),
}));

vi.mock('../../composables/useMcpCompletion', () => ({
  useMcpCompletion: () => ({
    supported: { value: false, __v_isRef: true },
    complete: mocks.complete,
  }),
}));

describe('McpPromptPicker', () => {
  beforeEach(() => {
    mocks.getPrompt.mockReset();
    mocks.connect.mockClear();
    mocks.connected.value = true;
  });

  it('resolves prompts through canonical getPrompt', async () => {
    mocks.getPrompt.mockResolvedValue({
      description: 'Resolved prompt',
      messages: [{ role: 'user', content: { type: 'text', text: 'hello' } }],
    });

    const wrapper = mount(McpPromptPicker, {
      props: {
        open: true,
        command: {
          name: 'mcp.maenifold.github:summary',
          description: 'Summary prompt',
          _meta: {
            'maenifold.mcpPrompt': {
              kind: 'mcp-prompt',
              serverName: 'maenifold',
              promptName: 'github:summary',
              arguments: [],
            },
          },
        },
      },
      global: {
        stubs: {
          Button: { template: '<button data-slot="button" @click="$emit(\'click\')"><slot /></button>' },
          Dialog: { template: '<div><slot /></div>' },
          DialogContent: { template: '<div><slot /></div>' },
          DialogFooter: { template: '<div><slot /></div>' },
          DialogHeader: { template: '<div><slot /></div>' },
          DialogTitle: { template: '<div><slot /></div>' },
          SchemaFormRenderer: { template: '<div data-testid="schema-form" />' },
        },
      },
    });

    await wrapper.findAll('button')[1]!.trigger('click');

    expect(mocks.getPrompt).toHaveBeenCalledWith('github:summary', {});
    expect(wrapper.emitted('resolved')).toEqual([[
      [{ role: 'user', content: { type: 'text', text: 'hello' } }],
    ]]);
    expect(wrapper.emitted('update:open')).toEqual([[false]]);
  });
});
