// Session persistence composable — owns savedSessions and browser persistence
import { computed, ref } from 'vue';
import type { AcpClientBridge } from '../lib/acp-client';
import type { SavedSession } from '../lib/types';
import { parseUpdatedAt } from './session-utils';

const STORAGE_KEY = 'maenifold:sessions';
const appVersion = import.meta.env.VITE_APP_VERSION ?? '0.1.0';

function loadSavedSessions(): SavedSession[] {
  if (typeof window === 'undefined') {
    return [];
  }

  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw);
    const sessions = Array.isArray(parsed) ? parsed as SavedSession[] : [];
    for (const session of sessions) {
      if (session.supportsLoadSession === undefined) {
        session.supportsLoadSession = true;
      }
    }
    return sessions;
  } catch {
    return [];
  }
}

export function useSessionPersistence() {
  const savedSessions = ref<SavedSession[]>([]);

  function getAppVersion(): string {
    return appVersion;
  }

  const resumableSessions = computed(() =>
    savedSessions.value.filter(s => s.supportsLoadSession === true)
  );

  async function initStore() {
    savedSessions.value = loadSavedSessions();
  }

  async function saveSessionsToStore() {
    if (typeof window === 'undefined') {
      return;
    }

    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(savedSessions.value));
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
