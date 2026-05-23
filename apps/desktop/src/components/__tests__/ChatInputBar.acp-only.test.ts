// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { describe, expect, it, vi } from 'vitest';

import ChatInputBar from '../ChatInputBar.vue';
import type { SlashCommandNotice } from '../../lib/slash-command-notice';
import type { AvailableCommand } from '../../lib/types';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

const commandPaletteStub = {
  name: 'CommandPalette',
  props: {
    commands: { type: Array, required: true },
    filter: { type: String, required: true },
    notice: { type: Object, default: null },
    visible: { type: Boolean, required: true },
  },
  template: `
    <div v-if="visible" data-test="command-palette">
      <div v-if="notice" data-test="command-notice">{{ notice.heading }} {{ notice.body }}</div>
      <div
        v-for="command in commands"
        :key="command.name"
        data-test="command-option"
      >
        {{ command.name }}
      </div>
    </div>
  `,
};

function mountInputBar(availableCommands: AvailableCommand[], commandPaletteNotice: SlashCommandNotice | null = null, modelValue = '/') {
  return mount(ChatInputBar, {
    props: {
      modelValue,
      placeholder: 'Message',
      disabled: false,
      availableCommands,
      commandPaletteNotice,
      supportsImageAttachments: true,
      supportsAudioAttachments: true,
      supportsEmbeddedContext: true,
      imageAttachmentTooltip: 'Attach image',
      audioAttachmentTooltip: 'Attach audio',
      embeddedContextTooltip: 'Embed file context',
      pendingAttachments: [],
      attachmentError: null,
    },
    global: {
      plugins: [createPinia()],
      stubs: {
        ChatAttachmentBar: true,
        CommandPalette: commandPaletteStub,
        VoiceInputButton: true,
      },
    },
  });
}

function renderedCommandNames(availableCommands: AvailableCommand[]): string[] {
  const wrapper = mountInputBar(availableCommands);
  const names = wrapper
    .findAll('[data-test="command-option"]')
    .map((option) => option.text());
  wrapper.unmount();
  return names;
}

describe('ChatInputBar ACP-advertised slash commands', () => {
  it('opens the palette notice on slash input without adding fake commands', () => {
    const wrapper = mountInputBar([], { heading: 'Slash commands unavailable', body: 'Copilot did not advertise slash commands over ACP.' });
    expect(wrapper.find('[data-test="command-palette"]').exists()).toBe(true);
    expect(wrapper.get('[data-test="command-notice"]').text()).toContain('Slash commands unavailable');
    expect(wrapper.findAll('[data-test="command-option"]')).toHaveLength(0);
  });

  it('renders only ACP-advertised slash commands in order', () => {
    expect(renderedCommandNames([
      { name: 'foo', description: 'Foo command' },
      { name: 'bar', description: 'Bar command' },
    ])).toEqual(['foo', 'bar']);
  });

  it('keeps the palette closed when no notice is present and no commands exist', () => {
    const wrapper = mountInputBar([], null, '/');
    expect(wrapper.find('[data-test="command-palette"]').exists()).toBe(false);
  });
});
