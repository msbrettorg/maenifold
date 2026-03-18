// Session store — thin orchestrator that composes focused composables
// while preserving the full external API contract unchanged.
import { defineStore } from 'pinia';
import { ref, computed, reactive } from 'vue';
import type {
  AgentInfo,
  SavedSession,
  ChatMessage,
  PlanEntry,
  PermissionRequest,
  SessionMode,
  SlashCommand,
  ModelInfo,
  SessionConfigOption,
  SessionState,
} from '../lib/types';
import { generateSessionId, createEmptySessionState } from '../lib/types';
import { SESSION_SET_MODEL_METHOD, type AcpClientBridge } from '../lib/acp-bridge';
import type { AuthMethod } from '@agentclientprotocol/sdk';
import { normalizeConfigOptions } from '../composables/session-utils';

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
  });

  // ── Computed shims (backward compatibility) ───────────────────────
  // These ensure all existing consumers (App.vue, ChatView.vue,
  // SessionList.vue) continue working without changes.

  const currentSession = computed<SavedSession | null>(
    () => activeSession.value?.session ?? null
  );

  const messages = computed<ChatMessage[]>(
    () => activeSession.value?.messages ?? []
  );

  const isConnected = computed<boolean>(
    () => activeSession.value?.isConnected ?? false
  );

  const isLoading = computed<boolean>(
    () => activeSession.value?.isLoading ?? false
  );

  const isConnecting = computed<boolean>(
    () => activeSession.value?.isConnecting ?? false
  );

  const error = computed<string | null>(
    () => activeSession.value?.error ?? null
  );

  const pendingPermission = computed<PermissionRequest | null>(
    () => activeSession.value?.pendingPermissions[0] ?? null
  );

  const supportsSessionList = computed<boolean>(
    () => activeSession.value?.supportsSessionList ?? false
  );

  const agentInfo = computed<AgentInfo | null>(
    () => activeSession.value?.agentInfo ?? null
  );

  const agentPromptCapabilities = computed<{
    image?: boolean;
    audio?: boolean;
    embeddedContext?: boolean;
  }>(() => activeSession.value?.agentPromptCapabilities ?? {});

  const pendingAuthMethods = computed<AuthMethod[]>(
    () => (activeSession.value?.pendingAuthMethods ?? []) as AuthMethod[]
  );

  const pendingAuthAgentName = computed<string>(
    () => activeSession.value?.pendingAuthAgentName ?? ''
  );

  const availableModes = computed<SessionMode[]>(
    () => activeSession.value?.availableModes ?? []
  );

  const currentModeId = computed<string>(
    () => activeSession.value?.currentModeId ?? ''
  );

  const configOptions = computed<SessionConfigOption[]>(
    () => activeSession.value?.configOptions ?? []
  );

  const hasConfigOptions = computed<boolean>(
    () => activeSession.value?.hasConfigOptions ?? false
  );

  const availableCommands = computed<SlashCommand[]>(() => {
    const cmds = activeSession.value?.availableCommands ?? [];
    return cmds.map((cmd) => ({
      name: cmd.name,
      description: cmd.description,
      hint: cmd.input?.hint ?? undefined,
      _meta: cmd._meta ?? null,
    }));
  });

  const currentPlan = computed<PlanEntry[] | null>(
    () => activeSession.value?.currentPlan ?? null
  );

  const availableModels = computed<ModelInfo[]>(
    () => activeSession.value?.availableModels ?? []
  );

  const currentModelId = computed<string>(
    () => activeSession.value?.currentModelId ?? ''
  );

  const startupPhase = computed<string>(
    () => activeSession.value?.startupPhase ?? ''
  );

  const startupLogs = computed<string[]>(
    () => activeSession.value?.startupLogs ?? []
  );

  const startupElapsed = computed<number>(
    () => activeSession.value?.startupElapsed ?? 0
  );

  // ── Derived computeds ─────────────────────────────────────────────
  const hasActiveSession = computed(() => currentSession.value !== null);
  const messageList = computed(() => messages.value);

  // ── Actions that remain in the orchestrator ───────────────────────

  async function setMode(modeId: string): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) throw new Error('No active session');
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) {
      throw new Error('No active session');
    }

    await bridge.setMode({
      sessionId: session.session.sessionId,
      modeId,
    });

    // Optimistically update the current mode
    session.currentModeId = modeId;
  }

  async function setModel(modelId: string): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) throw new Error('No active session');
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) {
      throw new Error('No active session');
    }
    if (!bridge.supportsAgentMethod(SESSION_SET_MODEL_METHOD)) {
      throw new Error('Active agent does not advertise support for _session/set_model');
    }

    await bridge.unstable_setSessionModel({
      sessionId: session.session.sessionId,
      modelId,
    });

    // Optimistically update the current model
    session.currentModelId = modelId;
  }

  async function setConfigOption(configId: string, value: string): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) throw new Error('No active session');
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) {
      throw new Error('No active session');
    }

    const response = await bridge.unstable_setSessionConfigOption({
      sessionId: session.session.sessionId,
      configId,
      value,
    });

    session.hasConfigOptions = true;
    session.configOptions = normalizeConfigOptions(response.configOptions);
    session.availableModes = [];
    session.currentModeId = '';
    session.availableModels = [];
    session.currentModelId = '';
  }

  // ── Error helpers ─────────────────────────────────────────────────

  function clearError() {
    const session = activeSession.value;
    if (session) {
      session.error = null;
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

    // Computed shims (backward compat)
    currentSession,
    messages,
    isConnected,
    isLoading,
    isConnecting,
    error,
    pendingPermission,
    supportsSessionList,
    agentInfo,
    agentPromptCapabilities,
    pendingAuthMethods,
    pendingAuthAgentName,
    availableModes,
    currentModeId,
    configOptions,
    hasConfigOptions,
    availableCommands,
    currentPlan,
    availableModels,
    currentModelId,
    startupPhase,
    startupLogs,
    startupElapsed,

    // Computed (derived)
    hasActiveSession,
    messageList,
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
