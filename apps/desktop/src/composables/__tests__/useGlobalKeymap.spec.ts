import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { effectScope } from 'vue';
import { useGlobalKeymap } from '../useGlobalKeymap';
import { useAppShellStore } from '../../stores/app-shell';

describe('useGlobalKeymap', () => {
  let pinia: ReturnType<typeof createPinia>;
  let scope: ReturnType<typeof effectScope>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    scope = effectScope();
  });

  afterEach(() => {
    scope.stop();
  });

  it('opens command palette on Meta+K', () => {
    scope.run(() => {
      useGlobalKeymap();
    });
    const store = useAppShellStore();
    expect(store.commandPaletteOpen).toBe(false);

    window.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'k',
      metaKey: true,
      bubbles: true,
    }));

    expect(store.commandPaletteOpen).toBe(true);
  });

  it('closes command palette on second Meta+K', () => {
    scope.run(() => {
      useGlobalKeymap();
    });
    const store = useAppShellStore();

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }));
    expect(store.commandPaletteOpen).toBe(true);

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }));
    expect(store.commandPaletteOpen).toBe(false);
  });

  it('removes listener when scope is disposed', () => {
    scope.run(() => {
      useGlobalKeymap();
    });
    scope.stop();

    const store = useAppShellStore();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }));
    expect(store.commandPaletteOpen).toBe(false);
  });

  it('ignores keydown without meta/ctrl modifier', () => {
    scope.run(() => {
      useGlobalKeymap();
    });
    const store = useAppShellStore();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', bubbles: true }));
    expect(store.commandPaletteOpen).toBe(false);
  });
});
