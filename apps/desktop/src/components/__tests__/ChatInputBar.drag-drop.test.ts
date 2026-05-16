// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia } from 'pinia';
import ChatInputBar from '../ChatInputBar.vue';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

vi.mock('../../lib/attachment-helpers', async () => {
  const actual = await vi.importActual('../../lib/attachment-helpers');
  return {
    ...actual,
    readFileAsBase64: vi.fn().mockResolvedValue('dGVzdA=='),
  };
});

const commandPaletteHandleKeyDown = vi.fn();
const mountedWrappers: VueWrapper[] = [];

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
    global: { plugins: [createPinia()], stubs },
  });
  mountedWrappers.push(wrapper);
  return wrapper;
}

function filesEvent(type: 'dragover' | 'dragleave' | 'drop', files: File[], types: string[] = ['Files']): DragEvent {
  const event = new Event(type, { bubbles: true, cancelable: true }) as DragEvent;
  Object.defineProperty(event, 'dataTransfer', {
    value: { files, types, dropEffect: 'none' },
    configurable: true,
  });
  return event;
}

function pasteEvent(files: File[]): ClipboardEvent {
  const event = new Event('paste', { bubbles: true, cancelable: true }) as ClipboardEvent;
  Object.defineProperty(event, 'clipboardData', {
    value: { files },
    configurable: true,
  });
  return event;
}

async function flushAttachments(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await nextTick();
}

afterEach(() => {
  for (const wrapper of mountedWrappers.splice(0)) {
    wrapper.unmount();
  }
});

describe('ChatInputBar drag-drop attachments', () => {
  it('emits add-attachments when files are dropped', async () => {
    const wrapper = mountInput();
    const file = new File(['test'], 'test.png', { type: 'image/png' });

    wrapper.element.dispatchEvent(filesEvent('drop', [file]));
    await flushAttachments();

    expect(wrapper.emitted('add-attachments')).toHaveLength(1);
  });

  it('shows drop zone overlay on dragover with files', async () => {
    const wrapper = mountInput();

    wrapper.element.dispatchEvent(filesEvent('dragover', []));
    await nextTick();

    expect(wrapper.text()).toContain('Drop files to attach');
  });

  it('hides drop zone overlay on dragleave', async () => {
    const wrapper = mountInput();

    wrapper.element.dispatchEvent(filesEvent('dragover', []));
    await nextTick();
    wrapper.element.dispatchEvent(filesEvent('dragleave', []));
    await nextTick();

    expect(wrapper.text()).not.toContain('Drop files to attach');
  });

  it('emits add-attachments on paste with files', async () => {
    const wrapper = mountInput();
    const file = new File(['test'], 'test.png', { type: 'image/png' });

    wrapper.find('textarea').element.dispatchEvent(pasteEvent([file]));
    await flushAttachments();

    expect(wrapper.emitted('add-attachments')).toHaveLength(1);
  });

  it('ignores paste without files', async () => {
    const wrapper = mountInput();

    wrapper.find('textarea').element.dispatchEvent(pasteEvent([]));
    await flushAttachments();

    expect(wrapper.emitted('add-attachments')).toBeUndefined();
  });

  it('ignores drop when disabled', async () => {
    const wrapper = mountInput({ disabled: true });
    const file = new File(['test'], 'test.png', { type: 'image/png' });

    wrapper.element.dispatchEvent(filesEvent('drop', [file]));
    await flushAttachments();

    expect(wrapper.emitted('add-attachments')).toBeUndefined();
  });
});
