// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { computed, defineComponent, nextTick, ref } from 'vue';
import { mount } from '@vue/test-utils';
import { useEscCancelWhileStreaming } from '../useEscCancelWhileStreaming';

let currentWrapper: ReturnType<typeof mount> | null = null;

afterEach(() => {
  currentWrapper?.unmount();
  currentWrapper = null;
});

function mountHarness(initialLoading = true) {
  const isLoading = ref(initialLoading);
  const onCancel = vi.fn<() => void>();
  currentWrapper = mount(defineComponent({
    setup() {
      useEscCancelWhileStreaming(computed(() => isLoading.value), onCancel);
      return () => null;
    },
  }));
  return { isLoading, onCancel };
}

function pressEscape(options: KeyboardEventInit = {}) {
  const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true, ...options });
  vi.spyOn(event, 'preventDefault');
  window.dispatchEvent(event);
  return event;
}

describe('useEscCancelWhileStreaming', () => {
  it('cancels on window Escape while streaming', () => {
    const { onCancel } = mountHarness(true);
    const event = pressEscape();
    expect(onCancel).toHaveBeenCalledOnce();
    expect(event.preventDefault).toHaveBeenCalledOnce();
  });

  it('ignores Escape when streaming ends', async () => {
    const { isLoading, onCancel } = mountHarness(true);
    isLoading.value = false;
    await nextTick();
    pressEscape();
    expect(onCancel).not.toHaveBeenCalled();
  });
});
