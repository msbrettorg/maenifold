// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { defineComponent, h, nextTick } from 'vue';
import ShortcutCheatsheet from '../ShortcutCheatsheet.vue';
import { useGlobalKeymap } from '../../composables/useGlobalKeymap';
import { useAppShellStore } from '../../stores/app-shell';

const mountedWrappers: VueWrapper[] = [];
const originalPlatform = Object.getOwnPropertyDescriptor(window.navigator, 'platform');

const KeymapHarness = defineComponent({
  name: 'KeymapHarness',
  setup() {
    useGlobalKeymap();
    return () => h('div', { 'data-testid': 'keymap-harness' });
  },
});

function mountKeymapHarness() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const wrapper = mount(KeymapHarness, {
    attachTo: document.body,
    global: { plugins: [pinia] },
  });
  const store = useAppShellStore(pinia);
  mountedWrappers.push(wrapper);
  return { wrapper, store };
}

function mountCheatsheet(visible = true) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = useAppShellStore(pinia);
  store.cheatsheetOpen = visible;
  const wrapper = mount(ShortcutCheatsheet, {
    attachTo: document.body,
    props: { visible },
    global: { plugins: [pinia] },
  });
  mountedWrappers.push(wrapper);
  return wrapper;
}

function mockPlatform(platform: string): void {
  Object.defineProperty(window.navigator, 'platform', {
    configurable: true,
    value: platform,
  });
}

function shortcutEvent(): KeyboardEvent {
  return new KeyboardEvent('keydown', {
    key: '?',
    shiftKey: true,
    bubbles: true,
    cancelable: true,
  });
}

afterEach(() => {
  for (const wrapper of mountedWrappers.splice(0)) {
    wrapper.unmount();
  }
  document.body.innerHTML = '';
  if (originalPlatform) {
    Object.defineProperty(window.navigator, 'platform', originalPlatform);
  }
});

describe('useGlobalKeymap', () => {
  it('opens the cheatsheet on Shift+?', async () => {
    const { wrapper, store } = mountKeymapHarness();

    expect(store.cheatsheetOpen).toBe(false);

    wrapper.element.dispatchEvent(shortcutEvent());
    await nextTick();

    expect(store.cheatsheetOpen).toBe(true);
  });

  it('does not open the cheatsheet when an input is focused', async () => {
    const { store } = mountKeymapHarness();
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.focus();

    expect(document.activeElement).toBe(input);

    input.dispatchEvent(shortcutEvent());
    await nextTick();

    expect(store.cheatsheetOpen).toBe(false);
  });
});

describe('ShortcutCheatsheet', () => {
  it('renders Cmd hints on Mac platforms', () => {
    mockPlatform('MacIntel');
    const wrapper = mountCheatsheet(true);

    expect(wrapper.text()).toContain('Cmd');
  });

  it('renders Ctrl hints on non-Mac platforms', () => {
    mockPlatform('Win32');
    const wrapper = mountCheatsheet(true);

    expect(wrapper.text()).toContain('Ctrl');
  });
});
