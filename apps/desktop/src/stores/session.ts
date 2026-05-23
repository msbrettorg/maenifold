import { getNotificationSink } from '@/lib/notification-sink';
import { AGENT_METHODS } from '@agentclientprotocol/sdk';
import { defineStore } from 'pinia';
import { computed, reactive, ref, watch } from 'vue';
import type { McpAppUiMetadata, SavedSession, SessionState } from '../lib/types';
import { createEmptySessionState, generateSessionId } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import {
  applyConfigOptionsUpdate,
  isActiveSessionContextCurrent,
  requireActiveSessionContext,
} from '../composables/session-state-helpers';
import { useMessageHandler } from '../composables/useMessageHandler';
import { usePermissions } from '../composables/usePermissions';
import { useSessionBridges } from '../composables/useSessionBridges';
import { useSessionLifecycle } from '../composables/useSessionLifecycle';
import { useSessionModels } from '../composables/useSessionModels';
import { useSessionPersistence } from '../composables/useSessionPersistence';
import { createSessionGetters } from './session-getters';
import { useMcpToolDiscoveryStore } from './mcp-tool-discovery';

interface FocusOrResumeRemoteSessionOptions { suppressErrors?: boolean }

export const useSessionStore = defineStore('session', () => {
  const mcpToolDiscoveryStore = useMcpToolDiscoveryStore();
  const sessions = ref<Map<string, SessionState>>(new Map());
  const activeSessionId = ref<string | null>(null);
  const activeSession = computed<SessionState | null>(() =>
    activeSessionId.value ? sessions.value.get(activeSessionId.value) ?? null : null);
  const connectionError = ref<string | null>(null);
  const sessionModels = useSessionModels();
  const bridgeStore = useSessionBridges();
  const bridges = bridgeStore.bridges.value;
  const {
    savedSessions, resumableSessions, initStore, saveSessionsToStore, deleteSession, mergeSessionsFromAgent, getAppVersion,
  } = useSessionPersistence();
  const {
    resolvePermission, cancelPermission, selectAuthMethod, cancelAuthSelection, promptForAuthMethod, cancelAuthResolver,
  } = usePermissions({ sessions, bridges, activeSessionId });

  function resolveToolAppUi(toolName: string | null | undefined): McpAppUiMetadata | null {
    if (!toolName) return null;
    for (const result of mcpToolDiscoveryStore.serverResults) {
      if (result.status !== 'ok') continue;
      const bareName = toolName.includes('.') ? toolName.split('.').pop() ?? toolName : toolName;
      const qualifiedName = `${result.server}.${bareName}`;
      const tool = result.tools.find((candidate) =>
        candidate.name === toolName || candidate.name === bareName || candidate.name === qualifiedName || candidate.name.endsWith(`.${bareName}`));
      if (tool?.appUi) {
        return {
          ...tool.appUi,
          originatingServer: result.server,
          permissions: tool.appUi.permissions ? [...tool.appUi.permissions] : undefined,
        };
      }
    }
    return null;
  }

  const {
    handleSessionUpdate, sendPrompt, cancelOperation, pushSystemMessage: _pushSystemMessage, cancelPendingToolCalls: _cancelPendingToolCalls,
  } = useMessageHandler({ sessions, bridges, activeSessionId, savedSessions, saveSessionsToStore, cancelPermission, resolveToolAppUi });

  const { createSession, resumeSession, disconnect, closeSession, setActiveSession, cancelConnection } = useSessionLifecycle({
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
    clearConnectionError,
    cancelPendingToolCalls: _cancelPendingToolCalls,
    pushSystemMessage: _pushSystemMessage,
  });

  const sessionGetters = createSessionGetters({
    activeSession,
    activeSessionId: computed(() => activeSessionId.value),
    copilotModelReasoningEfforts: sessionModels.modelReasoningEfforts,
  });

  watch(
    () => [activeSessionId.value, activeSession.value?.agentName ?? '', activeSession.value?.isConnected ?? false, activeSession.value?.session?.sessionId ?? ''] as const,
    ([localId]) => {
      const session = activeSession.value;
      if (!localId || !session) return;
      void sessionModels.ensureSessionModels(localId, session);
    },
    { immediate: true },
  );

  async function setConfigOption(configId: string, value: string | boolean): Promise<void> {
    const context = requireActiveSessionContext(activeSessionId, sessions, bridges);
    const { session, bridge } = context;
    const configOption = session.configOptions.find((option) => option.id === configId);
    const nextValue = configOption?.valueType === 'boolean' && typeof value === 'string' ? value === 'true' : value;
    let response: Awaited<ReturnType<AcpClientBridge['unstable_setSessionConfigOption']>>;
    try {
      response = await bridge.unstable_setSessionConfigOption({ sessionId: session.session.sessionId, configId, value: nextValue });
    } catch (error) {
      if (!session.error) session.error = error instanceof Error ? error.message : String(error);
      throw error;
    }
    if (isActiveSessionContextCurrent(context, sessions, bridges)) applyConfigOptionsUpdate(session, response.configOptions);
  }

  async function setSessionMode(modeId: string): Promise<void> {
    const context = requireActiveSessionContext(activeSessionId, sessions, bridges);
    const { session, bridge } = context;
    try {
      await (bridge as unknown as { sendRequest<T>(method: string, params?: unknown): Promise<T> }).sendRequest(
        AGENT_METHODS.session_set_mode,
        { sessionId: session.session.sessionId, modeId },
      );
    } catch (error) {
      if (!session.error) session.error = error instanceof Error ? error.message : String(error);
      throw error;
    }
    if (isActiveSessionContextCurrent(context, sessions, bridges)) session.currentModeId = modeId;
  }

  function findLocalSessionByRemoteSessionId(remoteSessionId: string): { localId: string; session: SessionState } | null {
    const matches = [...sessions.value.entries()]
      .filter(([, session]) => session.session?.sessionId === remoteSessionId)
      .sort(([leftLocalId, leftSession], [rightLocalId, rightSession]) => {
        if (leftSession.isConnected !== rightSession.isConnected) return Number(rightSession.isConnected) - Number(leftSession.isConnected);
        return Number(rightLocalId.split('-')[1] ?? 0) - Number(leftLocalId.split('-')[1] ?? 0);
      });
    const [localId, session] = matches[0] ?? [];
    return localId && session ? { localId, session } : null;
  }

  function reportReconnectError(message: string, options: FocusOrResumeRemoteSessionOptions): void {
    connectionError.value = message;
    if (!options.suppressErrors) setError(message);
  }

  function clearConnectionError(): void { connectionError.value = null; }

  async function discardReconnectAttempt(previousSessionIds: Set<string>, previousActiveSessionId: string | null): Promise<string | null> {
    let didMutate = false;
    let disconnectError: string | null = null;
    for (const localId of [...sessions.value.keys()]) {
      if (previousSessionIds.has(localId)) continue;
      sessions.value.delete(localId);
      const bridge = bridgeStore.takeBridge(localId);
      if (bridge) {
        try {
          await bridge.disconnect();
        } catch (error) {
          disconnectError = error instanceof Error ? error.message : String(error);
        }
      }
      didMutate = true;
    }
    if (didMutate) sessions.value = new Map(sessions.value);
    if (previousActiveSessionId && sessions.value.has(previousActiveSessionId)) {
      activeSessionId.value = previousActiveSessionId;
      return disconnectError;
    }
    if (!activeSessionId.value || !sessions.value.has(activeSessionId.value)) activeSessionId.value = [...sessions.value.keys()][0] ?? null;
    return disconnectError;
  }

  async function focusOrResumeRemoteSession(
    remoteSessionId: string,
    agentName?: string,
    options: FocusOrResumeRemoteSessionOptions = {},
  ): Promise<{ status: 'activated' | 'resumed' | 'missing' | 'error'; localSessionId?: string }> {
    const trimmedSessionId = remoteSessionId.trim();
    if (!trimmedSessionId) {
      reportReconnectError('Unable to reconnect: missing ACP session identifier.', options);
      return { status: 'missing' };
    }
    const localSession = findLocalSessionByRemoteSessionId(trimmedSessionId);
    if (localSession?.session.isConnected) {
      setActiveSession(localSession.localId);
      clearConnectionError();
      return { status: 'activated', localSessionId: localSession.localId };
    }
    const savedSession = savedSessions.value.find((session) => session.sessionId === trimmedSessionId && session.supportsLoadSession === true);
    if (!savedSession && !agentName) {
      reportReconnectError(`ACP session ${trimmedSessionId} is unavailable. The originating agent session may have expired.`, options);
      return { status: 'missing' };
    }

    const resumable = savedSession ?? {
      id: `pipeline-${trimmedSessionId}`,
      agentName: agentName!,
      sessionId: trimmedSessionId,
      title: 'Pipeline session',
      lastUpdated: Date.now(),
      cwd: '/',
      supportsLoadSession: true,
    } satisfies SavedSession;
    const previousSessionIds = new Set(sessions.value.keys());
    const previousActiveSessionId = activeSessionId.value;

    try {
      await resumeSession(resumable);
      clearConnectionError();
      return { status: 'resumed', localSessionId: findLocalSessionByRemoteSessionId(trimmedSessionId)?.localId ?? undefined };
    } catch (error) {
      const cleanupError = options.suppressErrors
        ? await discardReconnectAttempt(previousSessionIds, previousActiveSessionId)
        : null;
      const message = error instanceof Error
        ? `Failed to reconnect ACP session ${trimmedSessionId}: ${error.message}`
        : `Failed to reconnect ACP session ${trimmedSessionId}.`;
      reportReconnectError(cleanupError ? `${message} Cleanup also failed: ${cleanupError}` : message, options);
      return { status: 'error' };
    }
  }

  function clearError(): void {
    if (activeSession.value) activeSession.value.error = null;
  }

  function clearMessages(): void {
    if (!activeSession.value) return;
    activeSession.value.messages = [];
    activeSession.value.toolCalls.clear();
    activeSession.value.currentPlan = null;
  }

  function setError(msg: string): void {
    getNotificationSink().error(msg);
    if (activeSession.value) {
      activeSession.value.error = msg;
      return;
    }
    const localId = generateSessionId();
    const errorSession = reactive(createEmptySessionState(localId, '', ''));
    errorSession.error = msg;
    sessions.value.set(localId, errorSession);
    sessions.value = new Map(sessions.value);
    activeSessionId.value = localId;
  }

  return {
    sessions,
    activeSessionId,
    activeSession,
    connectionError,
    savedSessions,
    ...sessionGetters,
    resumableSessions,
    initStore,
    createSession,
    resumeSession,
    sendPrompt,
    cancelOperation,
    cancelConnection,
    resolvePermission,
    cancelPermission,
    selectAuthMethod,
    cancelAuthSelection,
    disconnect,
    deleteSession,
    setConfigOption,
    setSessionMode,
    focusOrResumeRemoteSession,
    setError,
    clearError,
    clearConnectionError,
    clearMessages,
    setActiveSession,
    closeSession,
    get acpClient() { return activeSessionId.value ? bridges.get(activeSessionId.value) ?? null : null; },
  };
});
