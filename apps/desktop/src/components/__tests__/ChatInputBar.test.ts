// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { mount, type VueWrapper } from '@vue/test-utils';
import ChatInputBar from '../ChatInputBar.vue';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

const commandPaletteHandleKeyDown = vi.fn();

const stubs = {
  ChatAttachmentBar: { template: '<div data-stub="chat-attachment-bar" />' },
  VoiceInputButton: { template: '<div data-stub="voice-input-button" />' },
  CommandPalette: defineComponent({
    name: 'CommandPalette',
    setup(_, { expose }) {
      expose({ handleKeyDown: commandPaletteHandleKeyDown });
      return () => h('div', { 'data-stub': 'command-palette' });
    },
  }),
};

function mountInput(props: Partial<InstanceType<typeof ChatInputBar>['$props']> = {}): VueWrapper {
  commandPaletteHandleKeyDown.mockClear();
  return mount(ChatInputBar, {
    props: {
      modelValue: 'hello',
      placeholder: '',
      disabled: false,
      availableCommands: [],
      supportsImageAttachments: false,
      supportsAudioAttachments: false,
      supportsEmbeddedContext: false,
      imageAttachmentTooltip: '',
      audioAttachmentTooltip: '',
      embeddedContextTooltip: '',
      pendingAttachments: [],
      attachmentError: null,
      ...props,
    },
    global: { stubs },
  });
}

function enterEvent(options: { isComposing?: boolean; keyCode?: number; shiftKey?: boolean } = {}) {
  const event = new KeyboardEvent('keydown', {
    key: 'Enter',
    shiftKey: options.shiftKey ?? false,
    bubbles: true,
    cancelable: true,
  });
  Object.defineProperty(event, 'isComposing', {
    value: options.isComposing ?? false,
    configurable: true,
  });
  if (options.keyCode !== undefined) {
    Object.defineProperty(event, 'keyCode', {
      value: options.keyCode,
      configurable: true,
    });
  }
  const preventDefault = vi.spyOn(event, 'preventDefault');
  return { event, preventDefault };
}

async function dispatchEnter(wrapper: VueWrapper, event: KeyboardEvent): Promise<void> {
  wrapper.find('textarea').element.dispatchEvent(event);
  await nextTick();
}

describe('ChatInputBar', () => {
  it('ignores composing Enter without sending, palette handling, or preventDefault', async () => {
    const submitWrapper = mountInput();
    const submitEvent = enterEvent({ isComposing: true });

    await dispatchEnter(submitWrapper, submitEvent.event);

    expect(submitWrapper.emitted('send')).toBeUndefined();
    expect(submitEvent.preventDefault).not.toHaveBeenCalled();

    const paletteWrapper = mountInput({ modelValue: '/' });
    const paletteEvent = enterEvent({ isComposing: true });

    await dispatchEnter(paletteWrapper, paletteEvent.event);

    expect(paletteWrapper.emitted('send')).toBeUndefined();
    expect(commandPaletteHandleKeyDown).not.toHaveBeenCalled();
    expect(paletteEvent.preventDefault).not.toHaveBeenCalled();
  });

  it('ignores legacy Safari IME Enter keyCode 229 without sending, palette handling, or preventDefault', async () => {
    const submitWrapper = mountInput();
    const submitEvent = enterEvent({ keyCode: 229 });

    await dispatchEnter(submitWrapper, submitEvent.event);

    expect(submitWrapper.emitted('send')).toBeUndefined();
    expect(submitEvent.preventDefault).not.toHaveBeenCalled();

    const paletteWrapper = mountInput({ modelValue: '/' });
    const paletteEvent = enterEvent({ keyCode: 229 });

    await dispatchEnter(paletteWrapper, paletteEvent.event);

    expect(paletteWrapper.emitted('send')).toBeUndefined();
    expect(commandPaletteHandleKeyDown).not.toHaveBeenCalled();
    expect(paletteEvent.preventDefault).not.toHaveBeenCalled();
  });

  it('submits non-empty text on Enter when not composing and not disabled', async () => {
    const wrapper = mountInput();
    const { event, preventDefault } = enterEvent({ isComposing: false });

    await dispatchEnter(wrapper, event);

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('send')).toHaveLength(1);
  });

  it('leaves Shift+Enter to the textarea default without sending', async () => {
    const wrapper = mountInput();
    const { event, preventDefault } = enterEvent({ shiftKey: true });

    await dispatchEnter(wrapper, event);

    expect(preventDefault).not.toHaveBeenCalled();
    expect(wrapper.emitted('send')).toBeUndefined();
  });
});
