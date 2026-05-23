// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import ChatMessageItem from '../ChatMessageItem.vue';
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

function makeChatMessage(role: ChatMessage['role']): ChatMessage {
  return {
    id: `message-${role}`,
    role,
    content: 'Regression coverage for left-aligned chat layout.',
    timestamp: Date.now(),
  };
}

async function mountMessageItem(role: ChatMessage['role']): Promise<VueWrapper> {
  const pinia = createPinia();
  setActivePinia(pinia);

  const wrapper = mount(ChatMessageItem, {
    attachTo: document.body,
    props: {
      message: makeChatMessage(role),
      expandedThoughtIds: [],
      expandedToolCallIds: [],
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

function outerRow(wrapper: VueWrapper) {
  return wrapper.get('[role="article"]');
}

function bubble(wrapper: VueWrapper) {
  return outerRow(wrapper).get('[class~="max-w-[80%]"]');
}

function avatarContainers(wrapper: VueWrapper) {
  return outerRow(wrapper).findAll('[class~="mt-1"][class~="shrink-0"]');
}

function alignmentClassSignature(wrapper: VueWrapper): string {
  return outerRow(wrapper)
    .classes()
    .filter(
      (className) =>
        className.startsWith('justify-') ||
        className.startsWith('items-') ||
        className.startsWith('flex'),
    )
    .join(' ');
}

describe('ChatMessageItem left-aligned layout regression coverage', () => {
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

  it('keeps the outer container left-aligned for user role', async () => {
    const wrapper = await mountMessageItem('user');
    const row = outerRow(wrapper);

    expect(row.classes()).toContain('justify-start');
    expect(row.classes()).not.toContain('justify-end');
    expect(row.classes()).not.toContain('justify-self-end');

    wrapper.unmount();
  });

  it('keeps the outer container left-aligned for assistant role', async () => {
    const wrapper = await mountMessageItem('assistant');

    expect(outerRow(wrapper).classes()).toContain('justify-start');

    wrapper.unmount();
  });

  it('uses identical alignment classes regardless of role', async () => {
    const userWrapper = await mountMessageItem('user');
    const assistantWrapper = await mountMessageItem('assistant');

    expect(alignmentClassSignature(userWrapper)).toBe(alignmentClassSignature(assistantWrapper));

    userWrapper.unmount();
    assistantWrapper.unmount();
  });

  it('renders the user icon in the leading avatar slot for user role', async () => {
    const wrapper = await mountMessageItem('user');
    const [leadingAvatar] = avatarContainers(wrapper);

    expect(leadingAvatar?.exists()).toBe(true);
    expect(leadingAvatar.find('.lucide-user').exists()).toBe(true);

    wrapper.unmount();
  });

  it('renders exactly one avatar for user role and keeps it before the bubble', async () => {
    const wrapper = await mountMessageItem('user');
    const avatars = avatarContainers(wrapper);
    const avatarElement = avatars[0]?.element;
    const bubbleElement = bubble(wrapper).element;

    expect(avatars).toHaveLength(1);
    expect(
      Boolean(
        avatarElement &&
          (avatarElement.compareDocumentPosition(bubbleElement) & Node.DOCUMENT_POSITION_FOLLOWING),
      ),
    ).toBe(true);

    wrapper.unmount();
  });

  it('preserves role tint while sharing the left-aligned layout', async () => {
    const userWrapper = await mountMessageItem('user');
    const assistantWrapper = await mountMessageItem('assistant');

    expect(bubble(userWrapper).classes()).toContain('bg-surface-high');
    expect(bubble(assistantWrapper).classes()).toContain('bg-surface-container');
    expect(bubble(assistantWrapper).classes()).not.toContain('bg-surface-high');

    userWrapper.unmount();
    assistantWrapper.unmount();
  });
});
