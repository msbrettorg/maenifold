// Session connection composable — manages local session slots and teardown flows.
import { reactive, watch, type Ref, type WatchStopHandle } from 'vue';
import type { SessionState } from '../lib/types';
import { createEmptySessionState, generateSessionId } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { trackEvent } from '../lib/telemetry';

interface UseSessionConnectionDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  cancelAuthResolver: (sessionId: string) => void;
}

interface AgentClosedHandlers {
  cancelPendingToolCalls: (session: SessionState) => void;
  pushSystemMessage: (session: SessionState, content: string) => void;
}

interface CreateSessionSlotOptions {
  startupPhase?: string;
}
export function useSessionConnection({
  sessions,
  bridges,
  activeSessionId,
  cancelAuthResolver,
}: UseSessionConnectionDeps) {
  const abortedSessions = new Set<string>();
  const permissionWatchStops = new Map<string, WatchStopHandle>();
  function replaceSessionsMap(): void {
    sessions.value = new Map(sessions.value);
  }
  function setSessionState(sessionId: string, sessionState: SessionState): void {
    sessions.value.set(sessionId, sessionState);
    replaceSessionsMap();
  }
  function deleteSessionState(sessionId: string): void {
    sessions.value.delete(sessionId);
    replaceSessionsMap();
  }
  function adjustActiveSessionId(removedId: string): void {
    if (activeSessionId.value === removedId) {
      const remaining = Array.from(sessions.value.keys());
      activeSessionId.value = remaining.length > 0 ? remaining[0] : null;
    }
  }
  function stopPermissionWatch(sessionId: string): void {
    const stop = permissionWatchStops.get(sessionId);
    if (!stop) {
      return;
    }

    stop();
    permissionWatchStops.delete(sessionId);
  }
  async function disconnectBridge(sessionId: string): Promise<void> {
    stopPermissionWatch(sessionId);
    const bridge = bridges.get(sessionId);
    bridges.delete(sessionId);
    if (!bridge) {
      return;
    }
    try {
      await bridge.disconnect();
    } catch (error) {
    }
  }
  async function closeRemoteSession(session: SessionState, bridge: AcpClientBridge): Promise<void> {
    if (!session.isConnected || !session.session || !bridge.supportsSessionClose()) return;
    try {
      await bridge.closeSession({ sessionId: session.session.sessionId });
    } catch (error) {
    }
  }
  function bindPendingPermission(sessionId: string, bridge: AcpClientBridge): void {
    stopPermissionWatch(sessionId);
    const stop = watch(
      () => bridge.pendingPermissionRequests.value,
      (value) => {
        const session = sessions.value.get(sessionId);
        if (!session) {
          return;
        }

        session.pendingPermissions = value.map((request) => ({
          ...request,
          toolCall: {
            ...request.toolCall,
            locations: request.toolCall.locations?.map((location) => ({ ...location })),
            content: request.toolCall.content?.map((item) => ({ ...item })),
          },
          options: request.options.map((option) => ({ ...option })),
        }));
      },
      { immediate: true }
    );
    permissionWatchStops.set(sessionId, stop);
  }
  function cleanupInactiveActiveSession(): void {
    if (!activeSessionId.value) {
      return;
    }
    const existing = sessions.value.get(activeSessionId.value);
    if (!existing || existing.isConnected) {
      return;
    }
    existing.agentId = null;
    deleteSessionState(activeSessionId.value);
    bridges.delete(activeSessionId.value);
  }
  function createSessionSlot(
    agentName: string,
    cwd: string,
    options: CreateSessionSlotOptions = {}
  ) {
    cleanupInactiveActiveSession();

    const localId = generateSessionId();
    const sessionState = reactive(createEmptySessionState(localId, agentName, cwd));
    sessionState.isLoading = true;
    sessionState.isConnecting = true;
    sessionState.currentPlan = null;
    sessionState.startupPhase = options.startupPhase ?? '';

    setSessionState(localId, sessionState);
    activeSessionId.value = localId;
    abortedSessions.delete(localId);
    return { localId, sessionState };
  }
  function markSessionAborted(sessionId: string): void {
    abortedSessions.add(sessionId);
  }
  function clearSessionAbort(sessionId: string): void {
    abortedSessions.delete(sessionId);
  }
  function isSessionAborted(sessionId: string): boolean {
    return abortedSessions.has(sessionId);
  }
  async function disconnect(sessionId?: string): Promise<void> {
    const targetId = sessionId ?? activeSessionId.value;
    if (!targetId) {
      return;
    }

    const session = sessions.value.get(targetId);
    const bridge = bridges.get(targetId);
    const agentName = session?.session?.agentName || session?.agentName || 'unknown';
    const sessionStart = session?.session?.lastUpdated || Date.now();
    const sessionDuration = Math.round((Date.now() - sessionStart) / 1000);
    if (session) {
      session.currentPlan = null;
    }
    if (bridge) {
      if (session) {
        await closeRemoteSession(session, bridge);
      }
      await disconnectBridge(targetId);
    }
    if (session) {
      session.agentId = null;
    }
    trackEvent('SessionDisconnected', {
      agentName,
      sessionDurationSeconds: String(sessionDuration),
      messageCount: String(session?.messages.length ?? 0),
    });
    deleteSessionState(targetId);
    adjustActiveSessionId(targetId);
  }
  function setActiveSession(sessionId: string): void {
    if (sessions.value.has(sessionId)) {
      activeSessionId.value = sessionId;
    }
  }
  async function closeSession(sessionId: string): Promise<void> {
    const session = sessions.value.get(sessionId);
    if (!session) {
      return;
    }

    if (session.isConnected || session.isConnecting) {
      await disconnect(sessionId);
      return;
    }

    stopPermissionWatch(sessionId);
    bridges.delete(sessionId);
    session.agentId = null;
    deleteSessionState(sessionId);
    adjustActiveSessionId(sessionId);
  }
  async function cancelConnection(): Promise<void> {
    const targetId = activeSessionId.value;
    if (!targetId) {
      return;
    }

    markSessionAborted(targetId);
    cancelAuthResolver(targetId);

    const session = sessions.value.get(targetId);
    const bridge = bridges.get(targetId);
    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }
    if (bridge) {
      await disconnectBridge(targetId);
    }
    if (session) {
      session.agentId = null;
    }
    deleteSessionState(targetId);
    adjustActiveSessionId(targetId);
  }
  async function handleAgentClosed(
    agentId: string,
    handlers?: Partial<AgentClosedHandlers>
  ): Promise<void> {
    const cancelPendingToolCalls =
      handlers?.cancelPendingToolCalls ??
      ((session: SessionState) => {
        for (const toolCall of session.toolCalls.values()) {
          if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
            toolCall.status = 'cancelled';
          }
        }
        for (const message of session.messages) {
          for (const toolCall of message.toolCalls ?? []) {
            if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
              toolCall.status = 'cancelled';
            }
          }
        }
      });
    const pushSystemMessage =
      handlers?.pushSystemMessage ??
      ((session: SessionState, content: string) => {
        session.messages.push({
          id: crypto.randomUUID(),
          role: 'system',
          content,
          timestamp: Date.now(),
        });
      });
    let targetLocalId: string | null = null;
    for (const [localId, session] of sessions.value.entries()) {
      if (session.agentId === agentId) {
        targetLocalId = localId;
        break;
      }
    }
    if (!targetLocalId) return;

    const session = sessions.value.get(targetLocalId);
    if (!session) return;

    cancelAuthResolver(targetLocalId);
    session.pendingAuthMethods = [];
    session.pendingAuthAgentName = '';
    session.pendingPermissions = [];
    cancelPendingToolCalls(session);
    session.currentPlan = null;
    session.configOptions = [];
    session.hasConfigOptions = false;
    session.usage = null;
    session.agentId = null;
    session.isConnected = false;
    session.isConnecting = false;
    session.isLoading = false;
    session.startupPhase = 'agent_closed';
    session.error = `${session.agentName} exited. Start a new session to continue.`;
    pushSystemMessage(session, `${session.agentName} exited. This session is now disconnected.`);
    await disconnectBridge(targetLocalId);
    replaceSessionsMap();
  }
  return {
    setSessionState,
    stopPermissionWatch,
    bindPendingPermission,
    createSessionSlot,
    markSessionAborted,
    clearSessionAbort,
    isSessionAborted,
    disconnect,
    closeSession,
    setActiveSession,
    cancelConnection,
    handleAgentClosed,
  };
}
