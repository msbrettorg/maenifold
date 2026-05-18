// @vitest-environment jsdom
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import ChatMessageItem from '../ChatMessageItem.vue';
import { getMarkdownIt } from '../../composables/useMarkdownIt';
import type { ChatMessage } from '../../lib/types';

const memoryApi = vi.hoisted(() => ({
  readMemory: vi.fn(),
  writeMemory: vi.fn(),
  listMemories: vi.fn(),
}));

vi.mock('../../composables/useMemoryApi', () => ({ useMemoryApi: () => memoryApi }));

async function settle(): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await flushPromises();
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 10));
  }
}

function makeChatMessage(content: string): ChatMessage {
  return {
    id: 'message-1',
    role: 'assistant',
    content,
    timestamp: Date.now(),
  };
}

async function mountMessageItem(isStreaming: boolean) {
  const pinia = createPinia();
  setActivePinia(pinia);

  const wrapper = mount(ChatMessageItem, {
    attachTo: document.body,
    props: {
      message: makeChatMessage('Inline math: $\\frac{a}{b}$'),
      expandedThoughtIds: [],
      expandedToolCallIds: [],
      isStreaming,
    },
    global: {
      plugins: [pinia],
      stubs: {
        ChatContentBlocks: { template: '<div data-testid="chat-content-blocks" />' },
        ChatThoughtSection: { template: '<div data-testid="chat-thought-section" />' },
        ChatToolCallList: { template: '<div data-testid="chat-tool-call-list" />' },
        MessageActionToolbar: { template: '<div data-testid="message-action-toolbar" />' },
        StopReasonBanner: { template: '<div data-testid="stop-reason-banner" />' },
      },
    },
  });

  await settle();
  return wrapper;
}

describe('ChatMessageItem markdown rendering', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    document.body.innerHTML = '';
    memoryApi.readMemory.mockResolvedValue({
      uri: 'memory://Shared Concept',
      title: 'Shared Concept',
      content: '[[Shared Concept]]',
      checksum: 'cs1',
      created: null,
      modified: null,
    });
    memoryApi.writeMemory.mockResolvedValue('memory://Created');
    memoryApi.listMemories.mockResolvedValue('');
  });

  it('renders streaming assistant text through the markdown pipeline instead of a raw pre block', async () => {
    const wrapper = await mountMessageItem(true);

    expect(wrapper.find('pre').exists()).toBe(false);
    expect(wrapper.find('article.prose').exists()).toBe(true);

    wrapper.unmount();
  });

  it('keeps non-streaming assistant text on the markdown renderer path', async () => {
    const wrapper = await mountMessageItem(false);

    expect(wrapper.find('pre').exists()).toBe(false);
    expect(wrapper.find('article.prose').exists()).toBe(true);

    wrapper.unmount();
  });
});
