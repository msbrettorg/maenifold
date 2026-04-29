// Session store — thin orchestrator that composes focused composables
// while preserving the full external API contract unchanged.
import { defineStore } from 'pinia';
import { ref, computed, reactive } from 'vue';
import { toast } from 'vue-sonner';
import type { SavedSession, SessionState } from '../lib/types';
import { generateSessionId, createEmptySessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { createSessionGetters } from './session-getters';
import {
  applyConfigOptionsUpdate,
  requireActiveSessionContext,
  isActiveSessionContextCurrent,
} from '../composables/session-state-helpers';

// Composables
import { useSessionPersistence } from '../composables/useSessionPersistence';
import { usePermissions } from '../composables/usePermissions';
import { useMessageHandler } from '../composables/useMessageHandler';
import { useSessionLifecycle } from '../composables/useSessionLifecycle';
import { useMcpToolDiscoveryStore } from './mcp-tool-discovery';
import type { McpAppUiMetadata } from '../lib/types';

// `_meta.traceparent`, `_meta.tracestate`, and `_meta.baggage` are reserved
// for W3C trace context and must not be repurposed for custom ACP metadata.

export const useSessionStore = defineStore('session', () => {
  const mcpToolDiscoveryStore = useMcpToolDiscoveryStore();
  // ── Core multi-session state (owned by orchestrator) ──────────────
  const sessions = ref<Map<string, SessionState>>(new Map());
  const activeSessionId = ref<string | null>(null);
  const connectionError = ref<string | null>(null);
  const bridges = new Map<string, AcpClientBridge>();

  // ── Active session helper ─────────────────────────────────────────
  const activeSession = computed<SessionState | null>(() => {
    if (!activeSessionId.value) return null;
    return sessions.value.get(activeSessionId.value) ?? null;
  });

  // ── Compose: persistence ──────────────────────────────────────────
  const {
    savedSessions,
    resumableSessions,
    initStore,
    saveSessionsToStore,
    deleteSession,
    mergeSessionsFromAgent,
    getAppVersion,
  } = useSessionPersistence();

  // ── Compose: permissions ──────────────────────────────────────────
  const {
    resolvePermission,
    cancelPermission,
    selectAuthMethod,
    cancelAuthSelection,
    promptForAuthMethod,
    cancelAuthResolver,
  } = usePermissions({ sessions, bridges, activeSessionId });

  function resolveToolAppUi(toolName: string | null | undefined): McpAppUiMetadata | null {
    if (!toolName) return null;
    for (const result of mcpToolDiscoveryStore.serverResults) {
      if (result.status !== 'ok') continue;
      const bareName = toolName.includes('.') ? toolName.split('.').pop() ?? toolName : toolName;
      const qualifiedName = `${result.server}.${bareName}`;
      const tool = result.tools.find((candidate) =>
        candidate.name === toolName ||
        candidate.name === bareName ||
        candidate.name === qualifiedName ||
        candidate.name.endsWith(`.${bareName}`)
      );
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

  // ── Compose: message handler ──────────────────────────────────────
  const {
    handleSessionUpdate,
    sendPrompt,
    cancelOperation,
    pushSystemMessage: _pushSystemMessage,
    cancelPendingToolCalls: _cancelPendingToolCalls,
  } = useMessageHandler({
    sessions,
    bridges,
    activeSessionId,
    savedSessions,
    saveSessionsToStore,
    cancelPermission,
    resolveToolAppUi,
  });

  // ── Compose: lifecycle ────────────────────────────────────────────
  const {
    performInitialization: _performInitialization,
    createSession,
    resumeSession,
    disconnect,
    closeSession,
    setActiveSession,
    cancelConnection,
  } = useSessionLifecycle({
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

  const sessionGetters = createSessionGetters({ activeSession });

  // ── Actions that remain in the orchestrator ───────────────────────

  async function setConfigOption(configId: string, value: string | boolean): Promise<void> {
    const context = requireActiveSessionContext(activeSessionId, sessions, bridges);
    const { session, bridge } = context;

    const configOption = session.configOptions.find((option) => option.id === configId);
    const nextValue =
      configOption?.valueType === 'boolean' && typeof value === 'string'
        ? value === 'true'
        : value;
    let response: Awaited<ReturnType<AcpClientBridge['unstable_setSessionConfigOption']>>;

    try {
      response = await bridge.unstable_setSessionConfigOption({
        sessionId: session.session.sessionId,
        configId,
        value: nextValue,
      });
    } catch (error) {
      if (!session.error) {
        session.error = error instanceof Error ? error.message : String(error);
      }
      throw error;
    }

    if (!isActiveSessionContextCurrent(context, sessions, bridges)) {
      return;
    }

    applyConfigOptionsUpdate(session, response.configOptions);
  }

  function findLocalSessionByRemoteSessionId(
    remoteSessionId: string
  ): { localId: string; session: SessionState } | null {
    const matches = [...sessions.value.entries()]
      .filter(([, session]) => session.session?.sessionId === remoteSessionId)
      .sort(([leftLocalId, leftSession], [rightLocalId, rightSession]) => {
        if (leftSession.isConnected !== rightSession.isConnected) {
          return Number(rightSession.isConnected) - Number(leftSession.isConnected);
        }

        const leftCreatedAt = Number(leftLocalId.split('-')[1] ?? 0);
        const rightCreatedAt = Number(rightLocalId.split('-')[1] ?? 0);
        return rightCreatedAt - leftCreatedAt;
      });

    const [localId, session] = matches[0] ?? [];
    return localId && session ? { localId, session } : null;
  }

  interface FocusOrResumeRemoteSessionOptions {
    suppressErrors?: boolean;
  }

  function reportReconnectError(
    message: string,
    options: FocusOrResumeRemoteSessionOptions,
  ): void {
    connectionError.value = message;
    if (!options.suppressErrors) {
      setError(message);
    }
  }

  function clearConnectionError(): void {
    connectionError.value = null;
  }

  function discardReconnectAttempt(
    previousSessionIds: Set<string>,
    previousActiveSessionId: string | null,
  ): void {
    let didMutate = false;
    for (const localId of [...sessions.value.keys()]) {
      if (previousSessionIds.has(localId)) {
        continue;
      }

      sessions.value.delete(localId);
      bridges.delete(localId);
      didMutate = true;
    }

    if (didMutate) {
      sessions.value = new Map(sessions.value);
    }

    if (previousActiveSessionId && sessions.value.has(previousActiveSessionId)) {
      activeSessionId.value = previousActiveSessionId;
      return;
    }

    if (activeSessionId.value && sessions.value.has(activeSessionId.value)) {
      return;
    }

    activeSessionId.value = [...sessions.value.keys()][0] ?? null;
  }

  async function focusOrResumeRemoteSession(
    remoteSessionId: string,
    agentName?: string,
    options: FocusOrResumeRemoteSessionOptions = {},
  ): Promise<{
    status: 'activated' | 'resumed' | 'missing' | 'error';
    localSessionId?: string;
  }> {
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

    const savedSession = savedSessions.value.find(
      (session) =>
        session.sessionId === trimmedSessionId && session.supportsLoadSession === true,
    );

    if (!savedSession) {
      // Pipeline sessions aren't in savedSessions — try direct load if we know the agent
      if (agentName) {
        const syntheticSession: SavedSession = {
          id: `pipeline-${trimmedSessionId}`,
          agentName,
          sessionId: trimmedSessionId,
          title: 'Pipeline session',
          lastUpdated: Date.now(),
          cwd: '/',
          supportsLoadSession: true,
        };
        const previousSessionIds = new Set(sessions.value.keys());
        const previousActiveSessionId = activeSessionId.value;
        try {
          await resumeSession(syntheticSession);
          clearConnectionError();
          return {
            status: 'resumed',
            localSessionId: findLocalSessionByRemoteSessionId(trimmedSessionId)?.localId ?? undefined,
          };
        } catch (error) {
          if (options.suppressErrors) {
            discardReconnectAttempt(previousSessionIds, previousActiveSessionId);
          }
          reportReconnectError(
            error instanceof Error
              ? `Failed to reconnect ACP session ${trimmedSessionId}: ${error.message}`
              : `Failed to reconnect ACP session ${trimmedSessionId}.`,
            options,
          );
          return { status: 'error' };
        }
      }

      reportReconnectError(
        `ACP session ${trimmedSessionId} is unavailable. The originating agent session may have expired.`,
        options,
      );
      return { status: 'missing' };
    }

    const previousSessionIds = new Set(sessions.value.keys());
    const previousActiveSessionId = activeSessionId.value;
    try {
      await resumeSession(savedSession);
      clearConnectionError();
      return {
        status: 'resumed',
        localSessionId: findLocalSessionByRemoteSessionId(trimmedSessionId)?.localId ?? undefined,
      };
    } catch (error) {
      if (options.suppressErrors) {
        discardReconnectAttempt(previousSessionIds, previousActiveSessionId);
      }
      reportReconnectError(
        error instanceof Error
          ? `Failed to reconnect ACP session ${trimmedSessionId}: ${error.message}`
          : `Failed to reconnect ACP session ${trimmedSessionId}.`,
        options,
      );
      return { status: 'error' };
    }
  }

  // ── Error helpers ─────────────────────────────────────────────────

  function clearError() {
    const session = activeSession.value;
    if (session) {
      session.error = null;
    }
  }

  function clearMessages() {
    const session = activeSession.value;
    if (session) {
      session.messages = [];
      session.toolCalls.clear();
      session.currentPlan = null;
    }
  }

  function setError(msg: string) {
    toast.error(msg);
    const session = activeSession.value;
    if (session) {
      session.error = msg;
    } else {
      const localId = generateSessionId();
      const errorSession = reactive(createEmptySessionState(localId, '', ''));
      errorSession.error = msg;
      sessions.value.set(localId, errorSession);
      sessions.value = new Map(sessions.value);
      activeSessionId.value = localId;
    }
  }

  // ── Return ────────────────────────────────────────────────────────

  return {
    // Multi-session state
    sessions,
    activeSessionId,
    activeSession,
    connectionError,

    // State (global)
    savedSessions,

    // Computed shims + derived computeds (backward compat)
    ...sessionGetters,
    resumableSessions,

    // Actions
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
    focusOrResumeRemoteSession,
    setError,
    clearError,
    clearConnectionError,
    clearMessages,

    // Multi-session actions
    setActiveSession,
    closeSession,

    // Expose client for permission handling
    get acpClient() {
      if (!activeSessionId.value) return null;
      return bridges.get(activeSessionId.value) ?? null;
    },
  };
});
