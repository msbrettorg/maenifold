import { computed, ref } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import { useChatShortcuts } from '../useChatShortcuts';

function makeEvent(key: string, options: { metaKey?: boolean; ctrlKey?: boolean } = {}): KeyboardEvent {
  const event = new KeyboardEvent('keydown', {
    key,
    metaKey: options.metaKey ?? false,
    ctrlKey: options.ctrlKey ?? false,
    bubbles: true,
    cancelable: true,
  });
  vi.spyOn(event, 'preventDefault');
  return event;
}

function setup(options: { text?: string; streaming?: boolean } = {}) {
  const text = ref(options.text ?? 'hello');
  const streaming = ref(options.streaming ?? false);
  const submit = vi.fn();
  const recallLast = vi.fn();
  const cancelTurn = vi.fn();
  const requestClear = vi.fn();
  const shortcuts = useChatShortcuts({
    submit,
    recallLast,
    cancelTurn,
    requestClear,
    isStreaming: computed(() => streaming.value),
    isInputEmpty: computed(() => !text.value.trim()),
  });
  return { ...shortcuts, submit, recallLast, cancelTurn, requestClear, text, streaming };
}

describe('useChatShortcuts', () => {
  it('submits non-empty text on Cmd+Enter', () => {
    const { handleShortcut, submit } = setup({ text: 'hello' });

    expect(handleShortcut(makeEvent('Enter', { metaKey: true }))).toBe(true);

    expect(submit).toHaveBeenCalledTimes(1);
  });

  it('does not submit empty text on Cmd+Enter', () => {
    const { handleShortcut, submit } = setup({ text: '   ' });

    expect(handleShortcut(makeEvent('Enter', { metaKey: true }))).toBe(false);

    expect(submit).not.toHaveBeenCalled();
  });

  it('recalls the last message on Up-arrow when input is empty', () => {
    const { handleShortcut, recallLast } = setup({ text: '' });

    expect(handleShortcut(makeEvent('ArrowUp'))).toBe(true);

    expect(recallLast).toHaveBeenCalledTimes(1);
  });

  it('does not recall on Up-arrow when input is non-empty', () => {
    const { handleShortcut, recallLast } = setup({ text: 'draft' });

    expect(handleShortcut(makeEvent('ArrowUp'))).toBe(false);

    expect(recallLast).not.toHaveBeenCalled();
  });

  it('cancels the current turn on Esc while streaming', () => {
    const { handleShortcut, cancelTurn } = setup({ streaming: true });

    expect(handleShortcut(makeEvent('Escape'))).toBe(true);

    expect(cancelTurn).toHaveBeenCalledTimes(1);
  });

  it('does not cancel the current turn on Esc when not streaming', () => {
    const { handleShortcut, cancelTurn } = setup({ streaming: false });

    expect(handleShortcut(makeEvent('Escape'))).toBe(false);

    expect(cancelTurn).not.toHaveBeenCalled();
  });

  it('requests clear on Cmd+L', () => {
    const { handleShortcut, requestClear } = setup();

    expect(handleShortcut(makeEvent('l', { metaKey: true }))).toBe(true);

    expect(requestClear).toHaveBeenCalledTimes(1);
  });

  it('treats Ctrl+Enter and Ctrl+L the same as Cmd variants', () => {
    const { handleShortcut, submit, requestClear } = setup();

    expect(handleShortcut(makeEvent('Enter', { ctrlKey: true }))).toBe(true);
    expect(handleShortcut(makeEvent('l', { ctrlKey: true }))).toBe(true);

    expect(submit).toHaveBeenCalledTimes(1);
    expect(requestClear).toHaveBeenCalledTimes(1);
  });
});
