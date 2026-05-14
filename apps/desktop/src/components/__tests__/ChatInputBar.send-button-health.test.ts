// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import ChatInputBar from '../ChatInputBar.vue';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

function mountInputBar(agentConnected?: boolean) {
  return mount(ChatInputBar, {
    props: {
      modelValue: 'hello',
      placeholder: 'Message',
      disabled: false,
      availableCommands: [],
      supportsImageAttachments: true,
      supportsAudioAttachments: true,
      supportsEmbeddedContext: true,
      imageAttachmentTooltip: 'Attach image',
      audioAttachmentTooltip: 'Attach audio',
      embeddedContextTooltip: 'Embed file context',
      pendingAttachments: [],
      attachmentError: null,
      agentConnected,
    },
    global: {
      stubs: {
        ChatAttachmentBar: true,
        CommandPalette: true,
        VoiceInputButton: true,
      },
    },
  });
}

describe('ChatInputBar send-button health', () => {
  it('renders a connected health dot', () => {
    const wrapper = mountInputBar(true);

    const dot = wrapper.get('[aria-label="Agent connected"]');
    expect(dot.classes()).toContain('bg-success');
  });

  it('renders a disconnected health dot', () => {
    const wrapper = mountInputBar(false);

    const dot = wrapper.get('[aria-label="Agent connection lost"]');
    expect(dot.classes()).toContain('bg-danger');
  });

  it('omits the health dot when agentConnected is not provided', () => {
    const wrapper = mountInputBar();

    expect(wrapper.find('[aria-label="Agent connected"]').exists()).toBe(false);
    expect(wrapper.find('[aria-label="Agent connection lost"]').exists()).toBe(false);
  });
});
