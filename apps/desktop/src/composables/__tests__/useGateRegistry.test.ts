// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, nextTick } from 'vue';
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

  mount(defineComponent({
    setup() {
      Object.assign(registry, useGateRegistry(() => seedRoot));
      return () => null;
    },
  }));

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

  it('falls back to localStorage when the Tauri store is unavailable', async () => {
    window.localStorage.setItem('preferences.json:gateRoots', JSON.stringify(['/alpha', '/beta']));
    window.localStorage.setItem('preferences.json:selectedGateRoot', JSON.stringify('/beta'));

    vi.doMock('@tauri-apps/plugin-store', () => ({
      load: vi.fn().mockRejectedValue(new Error('tauri unavailable')),
    }));

    const registry = await loadRegistry();

    expect(registry.isReady.value).toBe(true);
    expect(registry.gateRoots.value).toEqual(['/alpha', '/beta']);
    expect(registry.selectedGateRoot.value).toBe('/beta');
    expect(registry.error.value).toBe('');
  });

  it('uses a browser prompt fallback when the Tauri dialog is unavailable', async () => {
    vi.doMock('@tauri-apps/plugin-store', () => ({
      load: vi.fn().mockRejectedValue(new Error('tauri unavailable')),
    }));
    vi.doMock('@tauri-apps/plugin-dialog', () => ({
      open: vi.fn().mockRejectedValue(new Error('dialog unavailable')),
    }));
    vi.spyOn(window, 'prompt').mockReturnValue('/browser/root');

    const registry = await loadRegistry();
    const selected = await registry.browseForRoot();

    expect(selected).toBe('/browser/root');
    expect(registry.selectedGateRoot.value).toBe('/browser/root');
    expect(registry.gateRoots.value).toEqual(['/browser/root']);
    expect(window.localStorage.getItem('preferences.json:selectedGateRoot')).toBe(JSON.stringify('/browser/root'));
  });
});
