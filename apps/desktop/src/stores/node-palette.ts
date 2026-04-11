import { ref } from 'vue';
import { defineStore } from 'pinia';

const STORAGE_KEY = 'pipeline-node-palette-v1';
export const NODE_PALETTE_DEFAULT_WIDTH = 380;
export const NODE_PALETTE_MIN_WIDTH = 320;
export const NODE_PALETTE_MAX_RECENTS = 10;

interface PersistedNodePaletteState {
  width?: unknown;
  recentTools?: unknown;
  pinnedTools?: unknown;
}

function clampWidth(value: number): number {
  if (!Number.isFinite(value)) {
    return NODE_PALETTE_DEFAULT_WIDTH;
  }

  return Math.max(NODE_PALETTE_MIN_WIDTH, Math.round(value));
}

function normalizeToolNames(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .filter((entry): entry is string => typeof entry === 'string')
    .map((entry) => entry.trim())
    .filter((entry, index, all) => entry.length > 0 && all.indexOf(entry) === index);
}

function loadPersistedState(): PersistedNodePaletteState {
  if (typeof window === 'undefined') {
    return {};
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as PersistedNodePaletteState) : {};
  } catch {
    return {};
  }
}

export const useNodePaletteStore = defineStore('node-palette', () => {
  const persisted = loadPersistedState();
  const searchQuery = ref('');
  const panelWidth = ref(clampWidth(
    typeof persisted.width === 'number' ? persisted.width : NODE_PALETTE_DEFAULT_WIDTH,
  ));
  const selectedToolName = ref<string | null>(null);
  const recentTools = ref<string[]>(
    normalizeToolNames(persisted.recentTools).slice(0, NODE_PALETTE_MAX_RECENTS),
  );
  const pinnedTools = ref<string[]>(normalizeToolNames(persisted.pinnedTools));

  function persistState(): void {
    if (typeof window === 'undefined') {
      return;
    }

    try {
      window.localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          width: panelWidth.value,
          recentTools: recentTools.value,
          pinnedTools: pinnedTools.value,
        }),
      );
    } catch {
      // localStorage persistence is best-effort only.
    }
  }

  function setSearchQuery(value: string): void {
    searchQuery.value = value;
  }

  function clearSearchQuery(): void {
    searchQuery.value = '';
  }

  function setPanelWidth(value: number): void {
    panelWidth.value = clampWidth(value);
    persistState();
  }

  function setSelectedTool(name: string | null | undefined): void {
    const trimmed = typeof name === 'string' ? name.trim() : '';
    selectedToolName.value = trimmed || null;
  }

  function recordRecentTool(name: string): void {
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }

    recentTools.value = [
      trimmed,
      ...recentTools.value.filter((entry) => entry !== trimmed),
    ].slice(0, NODE_PALETTE_MAX_RECENTS);
    persistState();
  }

  function togglePinnedTool(name: string): void {
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }

    pinnedTools.value = pinnedTools.value.includes(trimmed)
      ? pinnedTools.value.filter((entry) => entry !== trimmed)
      : [trimmed, ...pinnedTools.value];
    persistState();
  }

  function isToolPinned(name: string): boolean {
    return pinnedTools.value.includes(name);
  }

  function pruneUnavailableTools(availableTools: Iterable<string>): void {
    const available = new Set(
      Array.from(availableTools)
        .filter((entry): entry is string => typeof entry === 'string')
        .map((entry) => entry.trim())
        .filter((entry) => entry.length > 0),
    );

    recentTools.value = recentTools.value.filter((entry) => available.has(entry));
    pinnedTools.value = pinnedTools.value.filter((entry) => available.has(entry));

    if (selectedToolName.value && !available.has(selectedToolName.value)) {
      selectedToolName.value = null;
    }

    persistState();
  }

  return {
    searchQuery,
    panelWidth,
    selectedToolName,
    recentTools,
    pinnedTools,
    setSearchQuery,
    clearSearchQuery,
    setPanelWidth,
    setSelectedTool,
    recordRecentTool,
    togglePinnedTool,
    isToolPinned,
    pruneUnavailableTools,
  };
});
