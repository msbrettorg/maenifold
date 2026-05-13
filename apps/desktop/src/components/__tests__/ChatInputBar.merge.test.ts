// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
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

describe('ChatInputBar command merging', () => {
  it('shows all built-in commands when the command palette is triggered with slash and no agent commands', () => {
    expect(renderedCommandNames([])).toEqual([
      'help',
      'clear',
      'sessions',
      'new',
      'quit',
    ]);
  });

  it('shows both built-in commands and an agent-advertised command', () => {
    expect(renderedCommandNames([
      { name: 'agent-cmd', description: 'agent command' },
    ])).toEqual([
      'help',
      'clear',
      'sessions',
      'new',
      'quit',
      'agent-cmd',
    ]);
  });
});
