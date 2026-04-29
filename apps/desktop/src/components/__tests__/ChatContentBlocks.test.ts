import { describe, expect, it } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import ChatContentBlocks from '../ChatContentBlocks.vue';
import ToolCallCardExtensions from '../ToolCallCardExtensions.vue';
import type { ContentBlock, ToolCallInfo } from '../../lib/types';

function createToolCall(content: ToolCallInfo['content']): ToolCallInfo {
  return {
    toolCallId: 'tool-call-svg-regression',
    title: 'Render content',
    kind: 'execute',
    status: 'completed',
    locations: [],
    content,
  };
}

async function settle(): Promise<void> {
  await flushPromises();
}

async function mountChatBlocks(blocks: ContentBlock[]) {
  setActivePinia(createPinia());
  const wrapper = mount(ChatContentBlocks, {
    props: { blocks },
    attachTo: document.body,
  });
  await settle();
  return wrapper;
}

async function mountExtensions(content: ToolCallInfo['content']) {
  setActivePinia(createPinia());
  const wrapper = mount(ToolCallCardExtensions, {
    props: {
      toolCall: createToolCall(content),
      expanded: true,
    },
    attachTo: document.body,
  });
  await settle();
  return wrapper;
}

describe('ChatContentBlocks rendering regression coverage', () => {
  it('image/svg+xml resource with text content renders inline', async () => {
    const wrapper = await mountChatBlocks([{
      type: 'resource',
      resource: {
        uri: 'file:///diagram.svg',
        mimeType: 'image/svg+xml',
        text: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" /></svg>',
      },
    } as ContentBlock]);

    expect(wrapper.find('svg').exists()).toBe(true);
    expect(wrapper.find('circle').exists()).toBe(true);
    wrapper.unmount();
  });

  it('image/svg+xml resource with blob content renders via data URI', async () => {
    const wrapper = await mountChatBlocks([{
      type: 'resource',
      resource: {
        uri: 'file:///diagram.svg',
        mimeType: 'image/svg+xml',
        blob: 'PHN2Zy8+',
      },
    } as ContentBlock]);

    const image = wrapper.find('img');
    expect(image.exists()).toBe(true);
    expect(image.attributes('src')).toBe('data:image/svg+xml;base64,PHN2Zy8+');
    wrapper.unmount();
  });

  it('image/svg+xml with missing content falls to download', async () => {
    const wrapper = await mountChatBlocks([{
      type: 'resource',
      resource: {
        uri: 'file:///diagram.svg',
        mimeType: 'image/svg+xml',
      },
    } as ContentBlock]);

    expect(wrapper.find('circle').exists()).toBe(false);
    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.text()).toContain('diagram.svg');
    wrapper.unmount();
  });

  it('terminal content block renders with output', async () => {
    const wrapper = await mountExtensions([{ type: 'terminal', output: 'hello world', exitCode: 0 } as never]);

    expect(wrapper.find('[data-testid="terminal-output"]').text()).toContain('hello world');
    wrapper.unmount();
  });

  it('diff content block renders unified diff', async () => {
    const wrapper = await mountExtensions([{ type: 'diff', oldText: 'a\n', newText: 'b\n', path: '/x' } as never]);

    const diff = wrapper.find('[data-testid="tool-call-diff"]');
    expect(diff.exists()).toBe(true);
    expect(diff.text()).toContain('--- /x');
    expect(diff.text()).toContain('+++ /x');
    expect(diff.text()).toContain('-a');
    expect(diff.text()).toContain('+b');
    wrapper.unmount();
  });
});
