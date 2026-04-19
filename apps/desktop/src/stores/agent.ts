import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useMcpResources } from '../composables/useMcpResources';

const AGENT_SESSIONS_URI = 'agent://sessions';

export interface AgentSessionSummary {
  sessionId: string;
  agentName: string;
  cwd: string;
  title?: string;
  createdAt?: string;
  lastActivity?: string;
  updatedAt?: string;
  status: string;
}

export interface AgentSessionDetailResource {
  session: AgentSessionSummary;
  generatedAt: string;
}

interface AgentSessionsResource {
  sessions: AgentSessionSummary[];
  errors?: unknown[];
  generatedAt: string;
}

function parseResourceJson<T>(text?: string): T | null {
  if (!text) {
    return null;
  }

  try {
    return JSON.parse(text) as T;
  } catch {
    return null;
  }
}

export const useAgentStore = defineStore('agent', () => {
  const mcpResources = useMcpResources();
  const activeSessions = ref<AgentSessionSummary[]>([]);
  const selectedSession = ref<AgentSessionSummary | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const activeSessionMap = computed(
    () => new Map(activeSessions.value.map((session) => [session.sessionId, session] as const)),
  );

  let unsubscribeSessions: (() => void) | null = null;
  let mountedConsumers = 0;

  async function refreshSessions() {
    loading.value = true;
    try {
      const resource = await mcpResources.readResource(AGENT_SESSIONS_URI);
      const payload = parseResourceJson<AgentSessionsResource>(resource.text);
      activeSessions.value = payload?.sessions ?? [];
      error.value = null;
    } catch (refreshError) {
      error.value = refreshError instanceof Error ? refreshError.message : String(refreshError);
    } finally {
      loading.value = false;
    }
  }

  async function refreshSelectedSession(sessionId: string | null | undefined) {
    if (!sessionId) {
      selectedSession.value = null;
      return;
    }

    try {
      const resource = await mcpResources.readResource(`agent://session/${sessionId}`);
      const payload = parseResourceJson<AgentSessionDetailResource>(resource.text);
      selectedSession.value = payload?.session ?? null;
      error.value = null;
    } catch (refreshError) {
      selectedSession.value = null;
      error.value = refreshError instanceof Error ? refreshError.message : String(refreshError);
    }
  }

  async function start() {
    mountedConsumers += 1;
    if (mountedConsumers > 1) {
      return;
    }

    await refreshSessions();
    unsubscribeSessions = await mcpResources.subscribeResource(AGENT_SESSIONS_URI, () => {
      void refreshSessions();
    });
  }

  async function stop() {
    mountedConsumers = Math.max(0, mountedConsumers - 1);
    if (mountedConsumers > 0) {
      return;
    }

    if (unsubscribeSessions) {
      unsubscribeSessions();
      unsubscribeSessions = null;
    }
  }

  return {
    activeSessions,
    activeSessionMap,
    selectedSession,
    loading,
    error,
    start,
    stop,
    refreshSessions,
    refreshSelectedSession,
  };
});
