// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, nextTick } from 'vue';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

function createStorageMock(): Storage {
  const storage = new Map<string, string>();

  return {
    get length() {
      return storage.size;
    },
    clear() {
      storage.clear();
    },
    getItem(key: string) {
      return storage.get(key) ?? null;
    },
    key(index: number) {
      return Array.from(storage.keys())[index] ?? null;
    },
    removeItem(key: string) {
      storage.delete(key);
    },
    setItem(key: string, value: string) {
      storage.set(key, value);
    },
  };
}

async function loadRegistry(seedRoot = '') {
  const { useGateRegistry } = await import('../useGateRegistry');
  const registry = {} as ReturnType<typeof useGateRegistry>;

  const pinia = createPinia();
  setActivePinia(pinia);

  mount(defineComponent({
    setup() {
      Object.assign(registry, useGateRegistry(() => seedRoot));
      return () => null;
    },
  }), {
    global: { plugins: [pinia] },
  });

  await flushPromises();
  await nextTick();
  return registry;
}

function clearPreferenceStorage() {
  const storage = window.localStorage as Storage & { clear?: () => void };
  if (typeof storage.clear === 'function') {
    storage.clear();
    return;
  }

  for (const key of Object.keys(storage)) {
    if (key.startsWith('preferences.json:')) {
      storage.removeItem(key);
    }
  }
}

describe('useGateRegistry', () => {
  beforeEach(() => {
    vi.resetModules();
    Object.defineProperty(window, 'localStorage', {
      value: createStorageMock(),
      configurable: true,
    });
    clearPreferenceStorage();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('loads roots from localStorage', async () => {
    window.localStorage.setItem('preferences.json:gateRoots', JSON.stringify(['/alpha', '/beta']));
    window.localStorage.setItem('preferences.json:selectedGateRoot', JSON.stringify('/beta'));

    const registry = await loadRegistry();

    expect(registry.isReady.value).toBe(true);
    expect(registry.gateRoots.value).toEqual(['/alpha', '/beta']);
    expect(registry.selectedGateRoot.value).toBe('/beta');
    expect(registry.error.value).toBe('');
  });

  // Removed: `pickAbsoluteDirectory` no longer falls back to `window.prompt`
  // when `HTMLDialogElement` is undefined. The directory picker now goes
  // through the `useFolderPickerStore` (Pinia), which is driven exclusively
  // by an in-app dialog UI. The old browser-prompt fallback path no longer
  // exists in production, so the test that asserted it has been removed.
});
