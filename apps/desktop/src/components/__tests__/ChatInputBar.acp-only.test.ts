// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { describe, expect, it, vi } from 'vitest';

import ChatInputBar from '../ChatInputBar.vue';
import type { AvailableCommand } from '../../lib/types';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

const commandPaletteStub = {
  name: 'CommandPalette',
  props: {
    commands: { type: Array, required: true },
    filter: { type: String, required: true },
    visible: { type: Boolean, required: true },
  },
  template: `
    <div v-if="visible" data-test="command-palette">
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

function mountInputBar(availableCommands: AvailableCommand[]) {
  return mount(ChatInputBar, {
    props: {
      modelValue: '/',
      placeholder: 'Message',
      disabled: false,
      availableCommands,
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
  it('renders no slash command entries when ACP advertises none', () => {
    expect(renderedCommandNames([])).toEqual([]);
  });

  it('renders only ACP-advertised slash commands in order', () => {
    expect(renderedCommandNames([
      { name: 'foo', description: 'Foo command' },
      { name: 'bar', description: 'Bar command' },
    ])).toEqual(['foo', 'bar']);
  });
});
