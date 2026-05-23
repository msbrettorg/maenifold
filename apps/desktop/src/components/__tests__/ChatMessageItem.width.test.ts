// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import ChatMessageItem from '../ChatMessageItem.vue';
import type { ChatMessage } from '../../lib/types';

function makeMessage(): ChatMessage {
  return {
    id: 'message-width-contract',
    role: 'assistant',
    content: 'A representative assistant message for width-contract assertions.',
    timestamp: Date.now(),
  };
}

describe('ChatMessageItem width contract', () => {
  it('allows the message bubble to fill at least 95% of the available chat width', () => {
    const wrapper = mount(ChatMessageItem, {
      props: {
        message: makeMessage(),
        expandedThoughtIds: [],
        expandedToolCallIds: [],
      },
      global: {
        stubs: {
          ChatContentBlocks: { template: '<div data-testid="chat-content-blocks" />' },
          ChatThoughtSection: { template: '<div data-testid="chat-thought-section" />' },
          ChatToolCallList: { template: '<div data-testid="chat-tool-call-list" />' },
          MarkdownView: { template: '<article data-testid="markdown-view" />' },
          MessageActionToolbar: { template: '<div data-testid="message-action-toolbar" />' },
          StopReasonBanner: { template: '<div data-testid="stop-reason-banner" />' },
        },
      },
    });

    const bubble = wrapper.get('[data-slot="card"]');
    const classes = bubble.classes();
    const maxWidthClasses = classes.filter((token) => token.startsWith('max-w-'));
    const bracketedPercentCaps = maxWidthClasses.flatMap((token) => {
      const match = token.match(/^max-w-\[(\d+)%\]$/);
      return match ? [Number(match[1])] : [];
    });
    const hasNinetyFiveOrWiderCap =
      maxWidthClasses.length === 0 ||
      classes.includes('max-w-full') ||
      classes.includes('max-w-[100%]') ||
      bracketedPercentCaps.some((value) => value >= 95);

    expect(classes).not.toContain('max-w-[80%]');
    expect(bracketedPercentCaps.every((value) => value >= 95)).toBe(true);
    expect(hasNinetyFiveOrWiderCap).toBe(true);

    wrapper.unmount();
  });
});
