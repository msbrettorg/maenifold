import { defineStore } from 'pinia';
import { ref } from 'vue';

export const PINNED_SESSIONS_STORAGE_KEY = 'maenifold.pinnedSessions';

function normalizeSessionIds(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .filter((entry): entry is string => typeof entry === 'string')
    .map((entry) => entry.trim())
    .filter((entry, index, all) => entry.length > 0 && all.indexOf(entry) === index);
}

function readPinnedSessionIds(): string[] {
  if (typeof window === 'undefined') {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(PINNED_SESSIONS_STORAGE_KEY);
    return raw ? normalizeSessionIds(JSON.parse(raw)) : [];
  } catch {
    return [];
  }
}

export const usePinnedSessionsStore = defineStore('pinned-sessions', () => {
  const pinnedSessionIds = ref<Set<string>>(new Set(readPinnedSessionIds()));

  function persistPinnedSessionIds(): void {
    if (typeof window === 'undefined') {
      return;
    }

    try {
      window.localStorage.setItem(
        PINNED_SESSIONS_STORAGE_KEY,
        JSON.stringify(Array.from(pinnedSessionIds.value)),
      );
    } catch {
      // localStorage persistence is best-effort only.
    }
  }

  function togglePinnedSession(sessionId: string): void {
    const trimmed = sessionId.trim();
    if (!trimmed) {
      return;
    }

    const next = new Set(pinnedSessionIds.value);
    if (next.has(trimmed)) {
      next.delete(trimmed);
    } else {
      next.add(trimmed);
    }

    pinnedSessionIds.value = next;
    persistPinnedSessionIds();
  }

  function isPinned(sessionId: string): boolean {
    return pinnedSessionIds.value.has(sessionId);
  }

  return {
    pinnedSessionIds,
    togglePinnedSession,
    isPinned,
  };
});
