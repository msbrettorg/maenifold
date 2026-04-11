// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

import {
  NODE_PALETTE_DEFAULT_WIDTH,
  NODE_PALETTE_MAX_RECENTS,
  NODE_PALETTE_MIN_WIDTH,
  useNodePaletteStore,
} from '../node-palette';

function installMockLocalStorage() {
  const storage = new Map<string, string>();
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
      clear: () => storage.clear(),
    },
  });
}

function createStore() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return useNodePaletteStore();
}

beforeEach(() => {
  installMockLocalStorage();
  window.localStorage.clear();
});

describe('useNodePaletteStore', () => {
  it('persists panel width with minimum clamping', () => {
    const store = createStore();

    store.setPanelWidth(NODE_PALETTE_MIN_WIDTH - 20);
    expect(store.panelWidth).toBe(NODE_PALETTE_MIN_WIDTH);

    const reloaded = createStore();
    expect(reloaded.panelWidth).toBe(NODE_PALETTE_MIN_WIDTH);
  });

  it('records recent tools with dedupe and max history', () => {
    const store = createStore();

    for (let index = 0; index < NODE_PALETTE_MAX_RECENTS + 2; index += 1) {
      store.recordRecentTool(`server.Tool${index}`);
    }
    store.recordRecentTool('server.Tool3');

    expect(store.recentTools).toHaveLength(NODE_PALETTE_MAX_RECENTS);
    expect(store.recentTools[0]).toBe('server.Tool3');
    expect(new Set(store.recentTools).size).toBe(store.recentTools.length);
  });

  it('toggles pinned tools and restores persisted state', () => {
    const store = createStore();

    store.togglePinnedTool('maenifold.SearchMemories');
    expect(store.isToolPinned('maenifold.SearchMemories')).toBe(true);

    const reloaded = createStore();
    expect(reloaded.pinnedTools).toContain('maenifold.SearchMemories');

    reloaded.togglePinnedTool('maenifold.SearchMemories');
    expect(reloaded.isToolPinned('maenifold.SearchMemories')).toBe(false);
  });

  it('keeps transient query/selection in memory only', () => {
    const store = createStore();

    store.setSearchQuery('memory');
    store.setSelectedTool('maenifold.SearchMemories');
    expect(store.searchQuery).toBe('memory');
    expect(store.selectedToolName).toBe('maenifold.SearchMemories');

    const reloaded = createStore();
    expect(reloaded.searchQuery).toBe('');
    expect(reloaded.selectedToolName).toBe(null);
    expect(reloaded.panelWidth).toBe(NODE_PALETTE_DEFAULT_WIDTH);
  });

  it('prunes stale tool references from recents, pins, and selection', () => {
    const store = createStore();

    store.recordRecentTool('maenifold.SearchMemories');
    store.recordRecentTool('filesystem.ReadFile');
    store.togglePinnedTool('maenifold.SearchMemories');
    store.setSelectedTool('filesystem.ReadFile');

    store.pruneUnavailableTools(['maenifold.SearchMemories']);

    expect(store.recentTools).toEqual(['maenifold.SearchMemories']);
    expect(store.pinnedTools).toEqual(['maenifold.SearchMemories']);
    expect(store.selectedToolName).toBe(null);
  });
});
