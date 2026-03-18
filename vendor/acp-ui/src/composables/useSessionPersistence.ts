// Session persistence composable — owns savedSessions and store persistence
import { ref, computed } from 'vue';
import { load, Store } from '@tauri-apps/plugin-store';
import { getVersion } from '@tauri-apps/api/app';
import type { SavedSession } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-bridge';
import { parseUpdatedAt } from './session-utils';

const STORE_PATH = 'sessions.json';

export function useSessionPersistence() {
  const savedSessions = ref<SavedSession[]>([]);
  let store: Store | null = null;

  // App version (loaded once at startup, exposed for other composables)
  let appVersion = '0.1.0';

  function getAppVersion(): string {
    return appVersion;
  }

  const resumableSessions = computed(() =>
    savedSessions.value.filter(s => s.supportsLoadSession === true)
  );

  async function initStore() {
    store = await load(STORE_PATH);
    const saved = await store.get<SavedSession[]>('sessions');
    if (saved) {
      savedSessions.value = saved;
    }

    // Load app version from Tauri
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.warn('Failed to get app version:', e);
    }
  }

  async function saveSessionsToStore() {
    if (store) {
      await store.set('sessions', savedSessions.value);
      await store.save();
    }
  }

  async function deleteSession(sessionId: string): Promise<void> {
    savedSessions.value = savedSessions.value.filter(s => s.id !== sessionId);
    await saveSessionsToStore();
  }

  async function mergeSessionsFromAgent(
    bridge: AcpClientBridge,
    agentName: string,
    cwd: string
  ): Promise<void> {
    const listedSessions: Array<{
      sessionId: string;
      cwd: string;
      title?: string;
      updatedAt?: string;
    }> = [];

    let cursor: string | undefined;

    do {
      const response = await bridge.listSessions(
        cursor === undefined ? { cwd } : { cwd, cursor }
      );

      listedSessions.push(...response.sessions);
      cursor = response.nextCursor;
    } while (cursor !== undefined);

    const sessionsById = new Map(
      savedSessions.value.map((session) => [session.sessionId, session] as const)
    );

    for (const listedSession of listedSessions) {
      const existingSession = sessionsById.get(listedSession.sessionId);
      const nextTitle =
        listedSession.title ??
        existingSession?.title ??
        'Untitled session';
      const nextLastUpdated =
        parseUpdatedAt(listedSession.updatedAt) ??
        existingSession?.lastUpdated ??
        Date.now();

      if (existingSession) {
        existingSession.agentName = agentName;
        existingSession.cwd = listedSession.cwd;
        existingSession.title = nextTitle;
        existingSession.lastUpdated = nextLastUpdated;
        existingSession.supportsLoadSession = true;
        continue;
      }

      sessionsById.set(listedSession.sessionId, {
        id: `agent:${agentName}:${listedSession.sessionId}`,
        agentName,
        sessionId: listedSession.sessionId,
        title: nextTitle,
        lastUpdated: nextLastUpdated,
        cwd: listedSession.cwd,
        supportsLoadSession: true,
      });
    }

    savedSessions.value = Array.from(sessionsById.values());
    await saveSessionsToStore();
  }

  return {
    savedSessions,
    resumableSessions,
    initStore,
    saveSessionsToStore,
    deleteSession,
    mergeSessionsFromAgent,
    getAppVersion,
  };
}
