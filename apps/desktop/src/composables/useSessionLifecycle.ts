// Session lifecycle composable — composition root for startup and connection flows.
import type { SessionState } from '../lib/types';
import type { SessionRuntimeDeps } from './session-runtime-deps';
import { useSessionConnection } from './useSessionConnection';
import { useSessionStartup } from './useSessionStartup';

interface UseSessionLifecycleDeps extends SessionRuntimeDeps {
  cancelAuthResolver: (sessionId: string) => void;
  cancelPendingToolCalls: (session: SessionState) => void;
  pushSystemMessage: (session: SessionState, content: string) => void;
}

export function useSessionLifecycle({
  sessions,
  bridges,
  activeSessionId,
  savedSessions,
  saveSessionsToStore,
  mergeSessionsFromAgent,
  handleSessionUpdate,
  promptForAuthMethod,
  cancelAuthResolver,
  getAppVersion,
  cancelPendingToolCalls,
  pushSystemMessage,
}: UseSessionLifecycleDeps) {
  const connection = useSessionConnection({
    sessions,
    bridges,
    activeSessionId,
    cancelAuthResolver,
  });
  const runtimeDeps: SessionRuntimeDeps = {
    sessions,
    bridges,
    activeSessionId,
    savedSessions,
    saveSessionsToStore,
    mergeSessionsFromAgent,
    handleSessionUpdate,
    promptForAuthMethod,
    getAppVersion,
  };
  const startup = useSessionStartup({
    ...runtimeDeps,
    bindPendingPermission: connection.bindPendingPermission,
    stopPermissionWatch: connection.stopPermissionWatch,
    createSessionSlot: connection.createSessionSlot,
    clearSessionAbort: connection.clearSessionAbort,
    isSessionAborted: connection.isSessionAborted,
  });

  const bridgeClosePoll = globalThis.setInterval(() => {
    for (const [localId, session] of sessions.value.entries()) {
      const bridge = bridges.get(localId);
      if (!bridge || !session.agentId || !bridge.isTransportClosed()) {
        continue;
      }

      void connection.handleAgentClosed(session.agentId, {
        cancelPendingToolCalls,
        pushSystemMessage,
      });
    }
  }, 500);

  void bridgeClosePoll;

  return {
    performInitialization: startup.performInitialization,
    createSession: startup.createSession,
    resumeSession: startup.resumeSession,
    disconnect: connection.disconnect,
    closeSession: connection.closeSession,
    setActiveSession: connection.setActiveSession,
    cancelConnection: connection.cancelConnection,
  };
}
