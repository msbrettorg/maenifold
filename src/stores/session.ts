// Session store — thin orchestrator that composes focused composables
// while preserving the full external API contract unchanged.
import { defineStore } from 'pinia';
import { ref, computed, reactive } from 'vue';
import type { SessionState } from '../lib/types';
import { generateSessionId, createEmptySessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { SESSION_SET_MODEL_METHOD } from '../lib/acp-meta';
import { createSessionGetters } from './session-getters';
import {
  applyConfigOptionsUpdate,
  isActiveSessionContextCurrent,
  requireActiveSessionContext,
} from '../composables/session-state-helpers';

// Composables
import { useSessionPersistence } from '../composables/useSessionPersistence';
import { usePermissions } from '../composables/usePermissions';
import { useMessageHandler } from '../composables/useMessageHandler';
import { useSessionLifecycle } from '../composables/useSessionLifecycle';

// `_meta.traceparent`, `_meta.tracestate`, and `_meta.baggage` are reserved
// for W3C trace context and must not be repurposed for custom ACP metadata.

export const useSessionStore = defineStore('session', () => {
  // ── Core multi-session state (owned by orchestrator) ──────────────
  const sessions = ref<Map<string, SessionState>>(new Map());
  const activeSessionId = ref<string | null>(null);
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
    cancelPendingToolCalls: _cancelPendingToolCalls,
    pushSystemMessage: _pushSystemMessage,
  });

  const sessionGetters = createSessionGetters({ activeSession });

  // ── Actions that remain in the orchestrator ───────────────────────

  async function setMode(modeId: string): Promise<void> {
    const context = requireActiveSessionContext(activeSessionId, sessions, bridges);
    const { session, bridge } = context;

    try {
      await bridge.setMode({
        sessionId: session.session.sessionId,
        modeId,
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

    session.currentModeId = modeId;
  }

  async function setModel(modelId: string): Promise<void> {
    const context = requireActiveSessionContext(activeSessionId, sessions, bridges);
    const { session, bridge } = context;
    if (!bridge.supportsAgentMethod(SESSION_SET_MODEL_METHOD)) {
      throw new Error('Active agent does not advertise support for _session/set_model');
    }

    try {
      await bridge.unstable_setSessionModel({
        sessionId: session.session.sessionId,
        modelId,
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

    session.currentModelId = modelId;
  }

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
    setMode,
    setModel,
    setConfigOption,
    setError,
    clearError,
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
