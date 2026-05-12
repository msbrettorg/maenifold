// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { computed, defineComponent, ref } from 'vue';
import { mount } from '@vue/test-utils';
import { useGateKeyboard } from '../useGateKeyboard';

type Cbs = {
  onNext: ReturnType<typeof vi.fn<() => void>>;
  onPrev: ReturnType<typeof vi.fn<() => void>>;
  onOpen: ReturnType<typeof vi.fn<() => void>>;
  onClose: ReturnType<typeof vi.fn<() => void>>;
  onNew: ReturnType<typeof vi.fn<() => void>>;
  onPass: ReturnType<typeof vi.fn<() => void>>;
  onFail: ReturnType<typeof vi.fn<() => void>>;
  onAppend: ReturnType<typeof vi.fn<() => void>>;
  onRefresh: ReturnType<typeof vi.fn<() => void>>;
  onHelp: ReturnType<typeof vi.fn<() => void>>;
};

let currentWrapper: ReturnType<typeof mount> | null = null;

afterEach(() => {
  currentWrapper?.unmount();
  currentWrapper = null;
});

function makeWrapper(opts: { isActive?: boolean; taskKey?: string; searchEl?: HTMLElement | null } = {}) {
  const { isActive = true, taskKey = 'task-key', searchEl = null } = opts;
  const cbs: Cbs = {
    onNext: vi.fn<() => void>(), onPrev: vi.fn<() => void>(), onOpen: vi.fn<() => void>(), onClose: vi.fn<() => void>(),
    onNew: vi.fn<() => void>(), onPass: vi.fn<() => void>(), onFail: vi.fn<() => void>(), onAppend: vi.fn<() => void>(),
    onRefresh: vi.fn<() => void>(), onHelp: vi.fn<() => void>(),
  };
  const isActiveRef = ref(isActive);
  currentWrapper = mount(defineComponent({
    setup() {
      useGateKeyboard({
        isActive: computed(() => isActiveRef.value),
        selectedTaskKey: ref(taskKey),
        searchInputEl: ref(searchEl),
        ...cbs,
      });
      return () => null;
    },
  }));
  return { cbs, isActiveRef };
}

function pressKey(key: string, extra: KeyboardEventInit = {}) {
  window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...extra }));
}

describe('useGateKeyboard', () => {
  it('j calls onNext', () => {
    const { cbs } = makeWrapper();
    pressKey('j');
    expect(cbs.onNext).toHaveBeenCalledOnce();
  });

  it('k calls onPrev', () => {
    const { cbs } = makeWrapper();
    pressKey('k');
    expect(cbs.onPrev).toHaveBeenCalledOnce();
  });

  it('Enter calls onOpen when a task is selected', () => {
    const { cbs } = makeWrapper({ taskKey: 'T-1' });
    pressKey('Enter');
    expect(cbs.onOpen).toHaveBeenCalledOnce();
  });

  it('Escape calls onClose', () => {
    const { cbs } = makeWrapper();
    pressKey('Escape');
    expect(cbs.onClose).toHaveBeenCalledOnce();
  });

  it('/ focuses the search input element', () => {
    const input = document.createElement('input');
    document.body.appendChild(input);
    makeWrapper({ searchEl: input });
    pressKey('/');
    expect(document.activeElement).toBe(input);
    document.body.removeChild(input);
  });

  it('n calls onNew', () => {
    const { cbs } = makeWrapper();
    pressKey('n');
    expect(cbs.onNew).toHaveBeenCalledOnce();
  });

  it('p calls onPass when a task is selected', () => {
    const { cbs } = makeWrapper({ taskKey: 'T-1' });
    pressKey('p');
    expect(cbs.onPass).toHaveBeenCalledOnce();
  });

  it('f calls onFail when a task is selected', () => {
    const { cbs } = makeWrapper({ taskKey: 'T-1' });
    pressKey('f');
    expect(cbs.onFail).toHaveBeenCalledOnce();
  });

  it('a calls onAppend when a task is selected', () => {
    const { cbs } = makeWrapper({ taskKey: 'T-1' });
    pressKey('a');
    expect(cbs.onAppend).toHaveBeenCalledOnce();
  });

  it('r calls onRefresh', () => {
    const { cbs } = makeWrapper();
    pressKey('r');
    expect(cbs.onRefresh).toHaveBeenCalledOnce();
  });

  it('? calls onHelp', () => {
    const { cbs } = makeWrapper();
    pressKey('?', { shiftKey: true });
    expect(cbs.onHelp).toHaveBeenCalledOnce();
  });

  it('does not fire callbacks when isActive is false', () => {
    const { cbs } = makeWrapper({ isActive: false });
    pressKey('j');
    pressKey('k');
    pressKey('n');
    expect(cbs.onNext).not.toHaveBeenCalled();
    expect(cbs.onPrev).not.toHaveBeenCalled();
    expect(cbs.onNew).not.toHaveBeenCalled();
  });

  it('does not fire callbacks when an input element is focused', () => {
    const { cbs } = makeWrapper();
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.focus();
    pressKey('j');
    expect(cbs.onNext).not.toHaveBeenCalled();
    input.blur();
    document.body.removeChild(input);
  });
});
