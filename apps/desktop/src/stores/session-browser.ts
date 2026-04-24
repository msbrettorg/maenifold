import { defineStore, storeToRefs } from 'pinia';
import { computed } from 'vue';
import { useAgentStore, type AgentSessionSummary } from './agent';

function isResumableSession(session: AgentSessionSummary): boolean {
  const normalizedStatus = session.status.trim().toLowerCase();
  return (
    normalizedStatus.includes('resumable') ||
    normalizedStatus.includes('load_session') ||
    normalizedStatus.includes('load-session') ||
    normalizedStatus.includes('load session') ||
    normalizedStatus.includes('loadsession')
  );
}

export const useSessionBrowserStore = defineStore('session-browser', () => {
  const agentStore = useAgentStore();
  const {
    activeSessions,
    loading: isLoading,
    error,
  } = storeToRefs(agentStore);

  const allSessions = computed(() => activeSessions.value);

  const sessionsByAgent = computed<Record<string, AgentSessionSummary[]>>(() => {
    return allSessions.value.reduce<Record<string, AgentSessionSummary[]>>((grouped, session) => {
      const agentSessions = grouped[session.agentName] ?? [];
      agentSessions.push(session);
      grouped[session.agentName] = agentSessions;
      return grouped;
    }, {});
  });

  const resumableSessions = computed(() =>
    allSessions.value.filter((session) => isResumableSession(session)),
  );

  async function refreshSessions() {
    await agentStore.refreshSessions();
  }

  function getSessionDetail(agentName: string, sessionId: string): AgentSessionSummary | null {
    return (
      sessionsByAgent.value[agentName]?.find((session) => session.sessionId === sessionId) ?? null
    );
  }

  return {
    isLoading,
    error,
    allSessions,
    sessionsByAgent,
    resumableSessions,
    refreshSessions,
    getSessionDetail,
  };
});
