// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia } from 'pinia';
import ChatInputBar from '../ChatInputBar.vue';
import { useAppShellStore } from '../../stores/app-shell';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

const commandPaletteHandleKeyDown = vi.fn();
const mountedWrappers: VueWrapper[] = [];
const keymapCleanups: Array<() => void> = [];

const stubs = {
  ChatAttachmentBar: { template: '<div data-stub="chat-attachment-bar" />' },
  VoiceInputButton: { template: '<div data-stub="voice-input-button" />' },
  CommandPalette: defineComponent({
    name: 'CommandPalette',
    props: {
      visible: { type: Boolean, required: true },
    },
    setup(props, { expose }) {
      expose({ handleKeyDown: commandPaletteHandleKeyDown });
      return () => props.visible ? h('div', { 'data-stub': 'command-palette' }) : null;
    },
  }),
};

function mountInput(
  props: Partial<InstanceType<typeof ChatInputBar>['$props']> = {},
  options: { attachTo?: HTMLElement } = {},
): VueWrapper {
  commandPaletteHandleKeyDown.mockClear();
  const pinia = createPinia();
  const wrapper = mount(ChatInputBar, {
    attachTo: options.attachTo,
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
    global: { plugins: [pinia], stubs },
  });
  const store = useAppShellStore(pinia);
  const handleGlobalKeymap = (event: KeyboardEvent): void => {
    if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'k') return;
    event.preventDefault();
    store.commandPaletteOpen = !store.commandPaletteOpen;
  };
  window.addEventListener('keydown', handleGlobalKeymap, true);
  keymapCleanups.push(() => window.removeEventListener('keydown', handleGlobalKeymap, true));
  mountedWrappers.push(wrapper);
  return wrapper;
}

function keyEvent(key: string, options: { isComposing?: boolean; keyCode?: number; shiftKey?: boolean; metaKey?: boolean } = {}) {
  const event = new KeyboardEvent('keydown', {
    key,
    shiftKey: options.shiftKey ?? false,
    metaKey: options.metaKey ?? false,
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

function enterEvent(options: { isComposing?: boolean; keyCode?: number; shiftKey?: boolean; metaKey?: boolean } = {}) {
  return keyEvent('Enter', options);
}

afterEach(() => {
  for (const cleanup of keymapCleanups.splice(0)) {
    cleanup();
  }
  for (const wrapper of mountedWrappers.splice(0)) {
    wrapper.unmount();
  }
});

async function dispatchEnter(wrapper: VueWrapper, event: KeyboardEvent): Promise<void> {
  wrapper.find('textarea').element.dispatchEvent(event);
  await nextTick();
}

async function dispatchKey(wrapper: VueWrapper, event: KeyboardEvent): Promise<void> {
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

  it('submits non-empty text on Cmd+Enter through the existing submit pathway', async () => {
    const wrapper = mountInput();
    const { event, preventDefault } = enterEvent({ metaKey: true });

    await dispatchEnter(wrapper, event);

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('send')).toHaveLength(1);
  });

  it('emits recall-last on Up-arrow with empty input', async () => {
    const wrapper = mountInput({ modelValue: '' });
    const { event, preventDefault } = keyEvent('ArrowUp');

    await dispatchKey(wrapper, event);

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('recall-last')).toHaveLength(1);
  });

  it('emits cancel-turn on Esc while streaming', async () => {
    const wrapper = mountInput({ isStreaming: true });
    const { event, preventDefault } = keyEvent('Escape');

    await dispatchKey(wrapper, event);

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('cancel-turn')).toHaveLength(1);
  });

  it('renders the stop icon button while loading', () => {
    const wrapper = mountInput({ disabled: true });

    expect(wrapper.find('button[aria-label="Cancel turn (Esc)"]').exists()).toBe(true);
  });

  it('does not render the stop icon button when idle', () => {
    const wrapper = mountInput({ disabled: false });

    expect(wrapper.find('button[aria-label="Cancel turn (Esc)"]').exists()).toBe(false);
  });

  it('opens the command palette on window Meta+K when input is empty', async () => {
    const wrapper = mountInput({
      modelValue: '   ',
      availableCommands: [{ name: 'acp-cmd', description: 'mock ACP command' }],
    });
    const { event, preventDefault } = keyEvent('k', { metaKey: true });

    window.dispatchEvent(event);
    await nextTick();

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('update:modelValue')).toEqual([['/']]);
    await wrapper.setProps({ modelValue: '/' });
    expect(wrapper.find('[data-stub="command-palette"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('opens the command palette on window Meta+K without replacing non-empty input', async () => {
    const draft = 'first paragraph\n\nsecond paragraph';
    const wrapper = mountInput({
      modelValue: draft,
      availableCommands: [{ name: 'acp-cmd', description: 'mock ACP command' }],
    });
    const { event, preventDefault } = keyEvent('k', { metaKey: true });

    expect(wrapper.find('[data-stub="command-palette"]').exists()).toBe(false);

    window.dispatchEvent(event);
    await nextTick();

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    expect(wrapper.find('[data-stub="command-palette"]').exists()).toBe(true);
    expect(wrapper.find('textarea').element.value).toBe(draft);
    wrapper.unmount();
  });

  it('opens a clear conversation AlertDialog on Cmd+L', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const wrapper = mountInput({}, { attachTo: host });
    const { event, preventDefault } = keyEvent('l', { metaKey: true });

    await dispatchKey(wrapper, event);
    await nextTick();

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(document.body.textContent).toContain('Clear conversation?');
    expect(document.body.textContent).toContain('This will remove all messages from this session. This cannot be undone.');
    wrapper.unmount();
    host.remove();
  });
});
