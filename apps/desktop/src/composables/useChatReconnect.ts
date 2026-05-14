import { computed, type ComputedRef } from 'vue';
import { useSessionStore } from '../stores/session';

export function useChatReconnect(): {
  canReconnect: ComputedRef<boolean>;
  reconnect: () => Promise<void>;
} {
  const sessionStore = useSessionStore();

  const canReconnect = computed<boolean>(() => {
    const saved = sessionStore.currentSession;
    return Boolean(
      saved?.sessionId &&
      saved?.supportsLoadSession === true &&
      !sessionStore.isLoading &&
      !sessionStore.isConnected,
    );
  });

  async function reconnect(): Promise<void> {
    const saved = sessionStore.currentSession;
    if (!saved?.sessionId) return;
    await sessionStore.focusOrResumeRemoteSession(
      saved.sessionId,
      saved.agentName,
      { suppressErrors: false },
    );
  }

  return { canReconnect, reconnect };
}
