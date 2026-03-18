// Session store for managing ACP sessions and persistence
import { defineStore } from 'pinia';
import { ref, computed, watch, type WatchStopHandle } from 'vue';
import { load, Store } from '@tauri-apps/plugin-store';
import { getVersion } from '@tauri-apps/api/app';
import { trackEvent, trackError } from '../lib/telemetry';
import type {
  SavedSession,
  ChatMessage,
  ToolCallInfo,
  PermissionRequest,
  SessionMode,
  SlashCommand,
  ModelInfo,
  SessionConfigOption,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
  SessionState,
  AvailableCommand,
} from '../lib/types';
import { generateSessionId, createEmptySessionState } from '../lib/types';
import { AcpClientBridge, createAcpClient } from '../lib/acp-bridge';
import { spawnAgent, killAgent, onAgentStderr } from '../lib/tauri';
import type { SessionNotification, AuthMethod } from '@agentclientprotocol/sdk';

const STORE_PATH = 'sessions.json';
const PROTOCOL_VERSION = 1;

// App version (loaded once at startup)
let appVersion = '0.1.0';

// Startup phase detection patterns
function detectPhase(line: string): string | null {
  const lower = line.toLowerCase();
  if (lower.includes('download') || lower.includes('fetch') || lower.includes('get ')) {
    return 'downloading';
  }
  if (lower.includes('install') || lower.includes('added') || lower.includes('packages')) {
    return 'installing';
  }
  if (lower.includes('build') || lower.includes('compil')) {
    return 'building';
  }
  if (lower.includes('start') || lower.includes('spawn')) {
    return 'starting';
  }
  return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function normalizeConfigSelectOption(option: unknown): SessionConfigSelectOption | null {
  if (!isRecord(option) || typeof option.value !== 'string' || typeof option.name !== 'string') {
    return null;
  }

  return {
    value: option.value,
    name: option.name,
    description: typeof option.description === 'string' ? option.description : undefined,
  };
}

function normalizeConfigSelectOptions(options: unknown): SessionConfigSelectOptions {
  if (!Array.isArray(options)) {
    return [];
  }

  const hasGroups = options.some(
    (option) => isRecord(option) && typeof option.group === 'string' && Array.isArray(option.options)
  );

  if (hasGroups) {
    return options
      .filter((option): option is Record<string, unknown> => isRecord(option))
      .map((group): SessionConfigSelectGroup | null => {
        if (
          typeof group.group !== 'string' ||
          typeof group.name !== 'string' ||
          !Array.isArray(group.options)
        ) {
          return null;
        }

        return {
          group: group.group,
          name: group.name,
          options: group.options
            .map(normalizeConfigSelectOption)
            .filter((option): option is SessionConfigSelectOption => option !== null),
        };
      })
      .filter((group): group is SessionConfigSelectGroup => group !== null);
  }

  return options
    .map(normalizeConfigSelectOption)
    .filter((option): option is SessionConfigSelectOption => option !== null);
}

function normalizeConfigOptions(configOptions: unknown): SessionConfigOption[] {
  if (!Array.isArray(configOptions)) {
    return [];
  }

  return configOptions
    .filter(
      (option): option is Record<string, unknown> =>
        isRecord(option) &&
        option.type === 'select' &&
        typeof option.id === 'string' &&
        typeof option.name === 'string' &&
        typeof option.currentValue === 'string'
    )
    .map((option) => ({
      id: option.id as string,
      name: option.name as string,
      description: typeof option.description === 'string' ? option.description : undefined,
      category: typeof option.category === 'string' ? option.category : undefined,
      type: 'select' as const,
      currentValue: option.currentValue as string,
      options: normalizeConfigSelectOptions(option.options),
    }));
}

function parseUpdatedAt(updatedAt?: string | null): number | undefined {
  if (!updatedAt) {
    return undefined;
  }

  const timestamp = new Date(updatedAt).getTime();
  return Number.isNaN(timestamp) ? undefined : timestamp;
}

export const useSessionStore = defineStore('session', () => {
  // ── Core multi-session state ──────────────────────────────────────
  const sessions = ref<Map<string, SessionState>>(new Map());
  const activeSessionId = ref<string | null>(null);
  const bridges = new Map<string, AcpClientBridge>();
  const permissionWatchStops = new Map<string, WatchStopHandle>();

  // ── Global state (not per-session) ────────────────────────────────
  const savedSessions = ref<SavedSession[]>([]);
  let store: Store | null = null;
  const abortedSessions = new Set<string>();
  const authMethodResolvers = new Map<string, (methodId: string | null) => void>();

  // ── Active session helper ─────────────────────────────────────────
  const activeSession = computed<SessionState | null>(() => {
    if (!activeSessionId.value) return null;
    return sessions.value.get(activeSessionId.value) ?? null;
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

  const agentInfo = computed<{ name: string; title?: string; version?: string } | null>(
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
    }));
  });

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
  const toolCallList = computed(() =>
    Array.from(activeSession.value?.toolCalls.values() ?? [])
  );
  const resumableSessions = computed(() =>
    savedSessions.value.filter(s => s.supportsLoadSession === true)
  );

  // ── Internal helpers (session-scoped) ─────────────────────────────

  type CancellableToolCallInfo = Omit<ToolCallInfo, 'status'> & {
    status: ToolCallInfo['status'] | 'cancelled';
  };

  function pushSystemMessage(session: SessionState, content: string): void {
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'system',
      content,
      timestamp: Date.now(),
    });
  }

  function cancelPendingToolCalls(session: SessionState): void {
    for (const toolCall of session.toolCalls.values()) {
      const cancellableToolCall = toolCall as CancellableToolCallInfo;
      if (
        cancellableToolCall.status === 'pending' ||
        cancellableToolCall.status === 'in_progress'
      ) {
        cancellableToolCall.status = 'cancelled';
      }
    }

    for (const message of session.messages) {
      if (!message.toolCalls) {
        continue;
      }

      for (const toolCall of message.toolCalls) {
        const cancellableToolCall = toolCall as CancellableToolCallInfo;
        if (
          cancellableToolCall.status === 'pending' ||
          cancellableToolCall.status === 'in_progress'
        ) {
          cancellableToolCall.status = 'cancelled';
        }
      }
    }
  }

  function applyConfigOptionsState(
    session: SessionState,
    response: { configOptions?: unknown[] | null }
  ): void {
    session.hasConfigOptions =
      'configOptions' in response && response.configOptions !== null;
    session.configOptions = session.hasConfigOptions
      ? normalizeConfigOptions(response.configOptions)
      : [];
  }

  function applyLegacySessionState(
    session: SessionState,
    response: {
      modes?: {
        availableModes?: Array<{ id: string; name: string; description?: string | null }>;
        currentModeId?: string | null;
      } | null;
      models?: {
        availableModels?: Array<{ modelId: string; name: string; description?: string | null }>;
        currentModelId?: string | null;
      } | null;
    }
  ): void {
    if (session.hasConfigOptions) {
      session.availableModes = [];
      session.currentModeId = '';
      session.availableModels = [];
      session.currentModelId = '';
      return;
    }

    if (response.modes) {
      session.availableModes = (response.modes.availableModes || []).map(m => ({
        id: m.id,
        name: m.name,
        description: m.description ?? undefined,
      }));
      session.currentModeId = response.modes.currentModeId || '';
    } else {
      session.availableModes = [];
      session.currentModeId = '';
    }

    if (response.models) {
      session.availableModels = (response.models.availableModels || []).map(m => ({
        modelId: m.modelId,
        name: m.name,
        description: m.description ?? undefined,
      }));
      session.currentModelId = response.models.currentModelId || '';
    } else {
      session.availableModels = [];
      session.currentModelId = '';
    }
  }

  function applySessionConfigurationState(
    session: SessionState,
    response: {
      configOptions?: unknown[] | null;
      modes?: {
        availableModes?: Array<{ id: string; name: string; description?: string | null }>;
        currentModeId?: string | null;
      } | null;
      models?: {
        availableModels?: Array<{ modelId: string; name: string; description?: string | null }>;
        currentModelId?: string | null;
      } | null;
    }
  ): void {
    applyConfigOptionsState(session, response);
    applyLegacySessionState(session, response);
  }

  async function performInitialization(client: AcpClientBridge) {
    const initResponse = await client.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: {
        fs: {
          readTextFile: true,
          writeTextFile: true,
        },
      },
      clientInfo: {
        name: 'maenifold',
        title: 'maenifold',
        version: appVersion,
      },
    });

    if (initResponse.protocolVersion !== PROTOCOL_VERSION) {
      await client.disconnect();
      throw new Error(
        `ACP protocol version mismatch: agent responded with version ${initResponse.protocolVersion}, expected ${PROTOCOL_VERSION}`
      );
    }

    const agentInfoResult = initResponse.agentInfo
      ? {
          name: initResponse.agentInfo.name,
          title: initResponse.agentInfo.title ?? undefined,
          version: initResponse.agentInfo.version ?? undefined,
        }
      : null;
    const promptCaps = initResponse.agentCapabilities?.promptCapabilities ?? null;

    return {
      initResponse,
      availableAuthMethods: initResponse.authMethods || [],
      agentInfoResult,
      promptCaps,
    };
  }

  // ── Store persistence ─────────────────────────────────────────────

  async function initStore() {
    store = await load(STORE_PATH);
    const saved = await store.get<SavedSession[]>('sessions');
    if (saved) {
      savedSessions.value = saved;
    }

    // Load app version from Tauri
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.warn('Failed to get app version:', e);
    }
  }

  async function saveSessionsToStore() {
    if (store) {
      await store.set('sessions', savedSessions.value);
      await store.save();
    }
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

  // ── Session update handler (scoped per session) ───────────────────

  function handleSessionUpdate(
    localSessionId: string,
    notification: SessionNotification
  ) {
    const session = sessions.value.get(localSessionId);
    if (!session) return;

    const update = notification.update;

    switch (update.sessionUpdate) {
      case 'user_message_chunk': {
        // Append to last user message or create new (for replay)
        const lastUserMsg = session.messages[session.messages.length - 1];
        if (lastUserMsg && lastUserMsg.role === 'user') {
          if (update.content.type === 'text') {
            lastUserMsg.content += update.content.text;
          }
        } else {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'user',
            content: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
          });
        }
        break;
      }

      case 'agent_message_chunk': {
        // Append to last assistant message or create new
        const lastMsg = session.messages[session.messages.length - 1];
        if (lastMsg && lastMsg.role === 'assistant') {
          if (update.content.type === 'text') {
            lastMsg.content += update.content.text;
          }
        } else {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
            toolCalls: [],
          });
        }
        break;
      }

      case 'agent_thought_chunk': {
        // Append to last assistant message's thought field or create new
        const lastAssistantMsg = session.messages[session.messages.length - 1];
        if (lastAssistantMsg && lastAssistantMsg.role === 'assistant') {
          if (update.content.type === 'text') {
            lastAssistantMsg.thought = (lastAssistantMsg.thought || '') + update.content.text;
          }
        } else {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: '',
            thought: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
            toolCalls: [],
          });
        }
        break;
      }

      case 'tool_call': {
        // Add tool call to the current assistant message
        const currentAssistantMsg = session.messages[session.messages.length - 1];
        if (currentAssistantMsg && currentAssistantMsg.role === 'assistant') {
          if (!currentAssistantMsg.toolCalls) {
            currentAssistantMsg.toolCalls = [];
          }
          currentAssistantMsg.toolCalls.push({
            toolCallId: update.toolCallId,
            title: update.title,
            kind: update.kind || 'other',
            status: update.status || 'pending',
            locations: update.locations,
          });
        }
        // Also keep in session's tool call map for updates
        session.toolCalls.set(update.toolCallId, {
          toolCallId: update.toolCallId,
          title: update.title,
          kind: update.kind || 'other',
          status: update.status || 'pending',
          locations: update.locations,
        });
        break;
      }

      case 'tool_call_update': {
        const existing = session.toolCalls.get(update.toolCallId);
        if (existing) {
          if (update.status) existing.status = update.status;
          if (update.title) existing.title = update.title;
          // Also update in the message's toolCalls array
          for (const msg of session.messages) {
            if (msg.toolCalls) {
              const tc = msg.toolCalls.find(t => t.toolCallId === update.toolCallId);
              if (tc) {
                if (update.status) tc.status = update.status;
                if (update.title) tc.title = update.title;
              }
            }
          }
        }
        break;
      }

      case 'current_mode_update':
        // Agent changed the mode
        if (session.hasConfigOptions) {
          break;
        }
        if ('modeId' in update && update.modeId) {
          session.currentModeId = update.modeId as string;
        }
        break;

      case 'config_option_update':
        session.configOptions = normalizeConfigOptions(update.configOptions);
        session.hasConfigOptions = true;
        session.availableModes = [];
        session.currentModeId = '';
        session.availableModels = [];
        session.currentModelId = '';
        break;

      case 'available_commands_update':
        // Agent advertised slash commands
        if ('availableCommands' in update && Array.isArray(update.availableCommands)) {
          session.availableCommands = update.availableCommands as AvailableCommand[];
        }
        break;

      case 'session_info_update': {
        const targetSessionId = notification.sessionId;
        const nextTitle =
          'title' in update
            ? update.title === null
              ? ''
              : update.title ?? undefined
            : undefined;
        const nextLastUpdated =
          'updatedAt' in update
            ? update.updatedAt === null
              ? Date.now()
              : parseUpdatedAt(update.updatedAt) ?? Date.now()
            : undefined;

        let savedSessionModified = false;
        const savedSession = savedSessions.value.find(
          (s) => s.sessionId === targetSessionId
        );
        if (savedSession) {
          if (nextTitle !== undefined) {
            savedSession.title = nextTitle;
            savedSessionModified = true;
          }
          if (nextLastUpdated !== undefined) {
            savedSession.lastUpdated = nextLastUpdated;
            savedSessionModified = true;
          }
        }

        if (session.session?.sessionId === targetSessionId) {
          if (nextTitle !== undefined) {
            session.session.title = nextTitle;
          }
          if (nextLastUpdated !== undefined) {
            session.session.lastUpdated = nextLastUpdated;
          }
        }

        if (savedSessionModified) {
          saveSessionsToStore();
        }
        break;
      }

      default:
        console.log('Unhandled session update:', update);
    }
  }

  // ── Auth flow ─────────────────────────────────────────────────────

  async function promptForAuthMethod(
    sessionId: string,
    session: SessionState,
    authMethods: AuthMethod[],
    agentName: string
  ): Promise<string | null> {
    return new Promise((resolve) => {
      session.pendingAuthMethods = authMethods;
      session.pendingAuthAgentName = agentName;
      authMethodResolvers.set(sessionId, resolve);
    });
  }

  function selectAuthMethod(methodId: string): void {
    const sessionId = activeSessionId.value;
    if (!sessionId) {
      return;
    }

    const resolver = authMethodResolvers.get(sessionId);
    if (resolver) {
      resolver(methodId);
      authMethodResolvers.delete(sessionId);
    }

    const session = activeSession.value;
    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }
  }

  function cancelAuthSelection(): void {
    const sessionId = activeSessionId.value;
    if (!sessionId) {
      return;
    }

    const resolver = authMethodResolvers.get(sessionId);
    if (resolver) {
      resolver(null);
      authMethodResolvers.delete(sessionId);
    }

    const session = activeSession.value;
    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }
  }

  // ── Helper to adjust activeSessionId after removal ────────────────

  function adjustActiveSessionId(removedId: string): void {
    if (activeSessionId.value === removedId) {
      const remaining = Array.from(sessions.value.keys());
      activeSessionId.value = remaining.length > 0 ? remaining[0] : null;
    }
  }

  function stopPermissionWatch(sessionId: string): void {
    const stop = permissionWatchStops.get(sessionId);
    if (stop) {
      stop();
      permissionWatchStops.delete(sessionId);
    }
  }

  function bindPendingPermission(sessionId: string, bridge: AcpClientBridge): void {
    stopPermissionWatch(sessionId);
    const stop = watch(
      () => bridge.pendingPermissionRequest.value,
      (value) => {
        const session = sessions.value.get(sessionId);
        if (session) {
          session.pendingPermissions = value ? [value] : [];
        }
      },
      { immediate: true }
    );
    permissionWatchStops.set(sessionId, stop);
  }

  // ── Create new session ────────────────────────────────────────────

  async function createSession(agentName: string, cwd: string): Promise<void> {
    // Clean up previous non-connected active session (e.g., from a failed attempt)
    if (activeSessionId.value) {
      const existing = sessions.value.get(activeSessionId.value);
      if (existing && !existing.isConnected) {
        sessions.value.delete(activeSessionId.value);
        bridges.delete(activeSessionId.value);
      }
    }

    // Create session slot
    const localId = generateSessionId();
    const sessionState = createEmptySessionState(localId, agentName, cwd);
    sessionState.isLoading = true;
    sessionState.isConnecting = true;
    sessionState.startupPhase = 'starting';
    sessions.value.set(localId, sessionState);
    activeSessionId.value = localId;

    abortedSessions.delete(localId);

    // Startup progress tracking (local to this call)
    let startupTimer: ReturnType<typeof setInterval> | null = null;
    let stderrUnlisten: (() => void) | null = null;

    startupTimer = setInterval(() => {
      const s = sessions.value.get(localId);
      if (s) s.startupElapsed++;
    }, 1000);

    try {
      // Spawn agent process
      const agentInstance = await spawnAgent(agentName);

      // Listen for stderr to track startup progress
      stderrUnlisten = await onAgentStderr((stderr) => {
        if (stderr.agent_id === agentInstance.id) {
          const s = sessions.value.get(localId);
          if (s) {
            s.startupLogs.push(stderr.line);
            const detectedPhase = detectPhase(stderr.line);
            if (detectedPhase) {
              s.startupPhase = detectedPhase;
            }
          }
        }
      }) as unknown as () => void;

      if (abortedSessions.has(localId)) {
        await killAgent(agentInstance.id);
        throw new Error('Connection cancelled');
      }

      sessionState.startupPhase = 'initializing';

      // Create ACP client bridge and store in bridges Map
      const bridge = await createAcpClient(agentInstance);
      bridges.set(localId, bridge);
      bindPendingPermission(localId, bridge);

      // Wire scoped session update handler
      bridge.onSessionUpdate = (notification) =>
        handleSessionUpdate(localId, notification);

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      sessionState.startupPhase = 'connecting';

      // Initialize connection
      const { initResponse, availableAuthMethods, agentInfoResult, promptCaps } =
        await performInitialization(bridge);

      console.log('Agent initialized:', initResponse);

      sessionState.agentInfo = agentInfoResult;
      sessionState.agentPromptCapabilities = promptCaps;

      // Check if agent supports session loading
      const supportsLoadSession = initResponse.agentCapabilities?.loadSession ?? false;
      sessionState.supportsSessionList = Boolean(
        initResponse.agentCapabilities?.sessionCapabilities?.list
      );

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      // Try to create session - may fail with auth_required
      let sessionResponse;
      try {
        sessionResponse = await bridge.newSession({
          cwd,
          mcpServers: [],
        });
      } catch (sessionError: unknown) {
        // Check if auth is required (error code -32000)
        const errorMessage = sessionError instanceof Error ? sessionError.message : String(sessionError);
        const isAuthRequired = errorMessage.toLowerCase().includes('authentication required') ||
                               errorMessage.includes('-32000');

        if (isAuthRequired && availableAuthMethods.length > 0) {
          console.log('Authentication required, available methods:', availableAuthMethods);

          // Prompt user to select auth method
          const selectedMethodId = await promptForAuthMethod(
            localId,
            sessionState, availableAuthMethods, agentName
          );

          if (!selectedMethodId || abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error('Authentication cancelled by user');
          }

          console.log('Authenticating with method:', selectedMethodId);
          const authResponse = await bridge.authenticate({
            methodId: selectedMethodId,
          });
          console.log('Authentication successful:', authResponse);

          if (abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error('Connection cancelled');
          }

          // Retry session creation after auth
          sessionResponse = await bridge.newSession({
            cwd,
            mcpServers: [],
          });
        } else {
          throw sessionError;
        }
      }

      // Save session
      const saved: SavedSession = {
        id: crypto.randomUUID(),
        agentName,
        sessionId: sessionResponse.sessionId,
        title: `Session ${new Date().toLocaleString()}`,
        lastUpdated: Date.now(),
        cwd,
        supportsLoadSession,
      };

      sessionState.session = saved;
      savedSessions.value.push(saved);
      await saveSessionsToStore();

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, agentName, cwd);
      }

      sessionState.isConnected = true;
      sessionState.messages = [];
      sessionState.toolCalls.clear();

      // Track successful session creation
      trackEvent('SessionCreated', { agentName, success: 'true' });

      applySessionConfigurationState(sessionState, sessionResponse);

    } catch (e) {
      const s = sessions.value.get(localId);
      if (s) {
        s.error = e instanceof Error ? e.message : String(e);
      }
      stopPermissionWatch(localId);
      bridges.delete(localId);
      // Track session creation failure
      trackEvent('SessionCreated', { agentName, success: 'false' });
      trackError(e instanceof Error ? e : new Error(String(e)));
      throw e;
    } finally {
      const s = sessions.value.get(localId);
      if (s) {
        s.isLoading = false;
        s.isConnecting = false;
      }
      abortedSessions.delete(localId);
      // Clean up startup progress tracking
      if (startupTimer) {
        clearInterval(startupTimer);
        startupTimer = null;
      }
      if (stderrUnlisten) {
        stderrUnlisten();
        stderrUnlisten = null;
      }
    }
  }

  // ── Resume existing session ───────────────────────────────────────

  async function resumeSession(savedSession: SavedSession): Promise<void> {
    for (const [localId, session] of sessions.value.entries()) {
      if (session.session?.sessionId === savedSession.sessionId) {
        activeSessionId.value = localId;
        return;
      }
    }

    // Clean up previous non-connected active session
    if (activeSessionId.value) {
      const existing = sessions.value.get(activeSessionId.value);
      if (existing && !existing.isConnected) {
        sessions.value.delete(activeSessionId.value);
        bridges.delete(activeSessionId.value);
      }
    }

    // Create session slot
    const localId = generateSessionId();
    const sessionState = createEmptySessionState(localId, savedSession.agentName, savedSession.cwd);
    sessionState.isLoading = true;
    sessionState.isConnecting = true;
    sessions.value.set(localId, sessionState);
    activeSessionId.value = localId;

    abortedSessions.delete(localId);

    try {
      // Spawn agent process
      const agentInstance = await spawnAgent(savedSession.agentName);

      if (abortedSessions.has(localId)) {
        await killAgent(agentInstance.id);
        throw new Error('Connection cancelled');
      }

      // Create ACP client bridge and store in bridges Map
      const bridge = await createAcpClient(agentInstance);
      bridges.set(localId, bridge);
      bindPendingPermission(localId, bridge);

      // Wire scoped session update handler
      bridge.onSessionUpdate = (notification) =>
        handleSessionUpdate(localId, notification);

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      // Initialize connection
      const { initResponse, availableAuthMethods, agentInfoResult, promptCaps } =
        await performInitialization(bridge);

      sessionState.agentInfo = agentInfoResult;
      sessionState.agentPromptCapabilities = promptCaps;

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      sessionState.supportsSessionList = Boolean(
        initResponse.agentCapabilities?.sessionCapabilities?.list
      );

      if (!initResponse.agentCapabilities?.loadSession) {
        await bridge.disconnect();
        throw new Error('Agent no longer supports session loading');
      }

      // Clear messages BEFORE loadSession - the agent will stream replay via notifications
      sessionState.messages = [];
      sessionState.toolCalls.clear();

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      // Try to load existing session - may fail with auth_required
      let loadResponse;
      try {
        loadResponse = await bridge.loadSession({
          sessionId: savedSession.sessionId,
          cwd: savedSession.cwd,
          mcpServers: [],
        });
      } catch (sessionError: unknown) {
        // Check if auth is required (error code -32000)
        const errorMessage = sessionError instanceof Error ? sessionError.message : String(sessionError);
        const isAuthRequired = errorMessage.toLowerCase().includes('authentication required') ||
                               errorMessage.includes('-32000');

        if (isAuthRequired && availableAuthMethods.length > 0) {
          console.log('Authentication required, available methods:', availableAuthMethods);

          // Prompt user to select auth method
          const selectedMethodId = await promptForAuthMethod(
            localId,
            sessionState, availableAuthMethods, savedSession.agentName
          );

          if (!selectedMethodId || abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error(
              abortedSessions.has(localId)
                ? 'Connection cancelled'
                : 'Authentication cancelled by user'
            );
          }

          console.log('Authenticating with method:', selectedMethodId);
          const authResponse = await bridge.authenticate({
            methodId: selectedMethodId,
          });
          console.log('Authentication successful:', authResponse);

          if (abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error('Connection cancelled');
          }

          // Retry loading session after auth
          loadResponse = await bridge.loadSession({
            sessionId: savedSession.sessionId,
            cwd: savedSession.cwd,
            mcpServers: [],
          });
        } else {
          throw sessionError;
        }
      }

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      sessionState.session = savedSession;
      sessionState.isConnected = true;
      // Messages already populated by session/update notifications during loadSession
      applySessionConfigurationState(sessionState, loadResponse);

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, savedSession.agentName, savedSession.cwd);
      }

      // Track successful session resume
      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'true' });

      // Update last accessed time
      savedSession.lastUpdated = Date.now();
      await saveSessionsToStore();

    } catch (e) {
      const s = sessions.value.get(localId);
      if (s) {
        s.error = e instanceof Error ? e.message : String(e);
      }
      stopPermissionWatch(localId);
      bridges.delete(localId);
      // Track session resume failure
      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'false' });
      trackError(e instanceof Error ? e : new Error(String(e)));
      throw e;
    } finally {
      const s = sessions.value.get(localId);
      if (s) {
        s.isLoading = false;
        s.isConnecting = false;
      }
      abortedSessions.delete(localId);
    }
  }

  // ── Send prompt ───────────────────────────────────────────────────

  async function sendPrompt(text: string): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) throw new Error('No active session');
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) {
      throw new Error('No active session');
    }

    // Add user message
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    });

    session.isLoading = true;
    try {
      const response = await bridge.prompt({
        sessionId: session.session.sessionId,
        prompt: [
          {
            type: 'text',
            text,
          },
        ],
      });

      console.log('Prompt completed:', response.stopReason);

      switch (response.stopReason) {
        case 'refusal':
          session.error = 'The agent refused this request.';
          break;
        case 'max_tokens':
          pushSystemMessage(session, 'Response was truncated due to token limits.');
          break;
        case 'max_turn_requests':
          pushSystemMessage(session, 'Turn limit reached.');
          break;
        case 'end_turn':
        case 'cancelled':
        default:
          break;
      }

      // Track prompt sent
      trackEvent('PromptSent', {
        messageLength: String(text.length),
        stopReason: response.stopReason || 'unknown',
      });

    } finally {
      session.isLoading = false;
    }
  }

  // ── Cancel current operation ──────────────────────────────────────

  async function cancelOperation(): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) return;
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) return;

    if (pendingPermission.value || bridge.pendingPermissionRequest.value) {
      cancelPermission();
    }

    cancelPendingToolCalls(session);

    await bridge.cancel({
      sessionId: session.session.sessionId,
    });
  }

  // ── Cancel ongoing connection attempt ─────────────────────────────

  async function cancelConnection(): Promise<void> {
    const targetId = activeSessionId.value;
    if (!targetId) return;
    abortedSessions.add(targetId);

    // Cancel auth selection if pending
    const resolver = authMethodResolvers.get(targetId);
    if (resolver) {
      resolver(null);
      authMethodResolvers.delete(targetId);
    }

    const session = sessions.value.get(targetId);
    const bridge = bridges.get(targetId);

    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }

    // Disconnect if bridge exists
    if (bridge) {
      try {
        await bridge.disconnect();
      } catch (e) {
        console.error('Error disconnecting:', e);
      }
      stopPermissionWatch(targetId);
      bridges.delete(targetId);
    }

    sessions.value.delete(targetId);
    adjustActiveSessionId(targetId);
  }

  // ── Permission handling ───────────────────────────────────────────

  function resolvePermission(optionId: string): void {
    if (!activeSessionId.value) return;
    const bridge = bridges.get(activeSessionId.value);
    if (bridge) {
      bridge.resolvePermission(optionId);
    }
  }

  function cancelPermission(): void {
    if (!activeSessionId.value) return;
    const bridge = bridges.get(activeSessionId.value);
    if (bridge) {
      bridge.cancelPermission();
    }
  }

  // ── Disconnect session ────────────────────────────────────────────

  async function disconnect(sessionId?: string): Promise<void> {
    const targetId = sessionId ?? activeSessionId.value;
    if (!targetId) return;

    const session = sessions.value.get(targetId);
    const bridge = bridges.get(targetId);

    const agentName = session?.session?.agentName || session?.agentName || 'unknown';
    const sessionStart = session?.session?.lastUpdated || Date.now();
    const sessionDuration = Math.round((Date.now() - sessionStart) / 1000);

    if (bridge) {
      await bridge.disconnect();
      stopPermissionWatch(targetId);
      bridges.delete(targetId);
    }

    // Track session disconnect
    trackEvent('SessionDisconnected', {
      agentName,
      sessionDurationSeconds: String(sessionDuration),
      messageCount: String(session?.messages.length ?? 0),
    });

    sessions.value.delete(targetId);
    adjustActiveSessionId(targetId);
  }

  // ── Set active session ────────────────────────────────────────────

  function setActiveSession(sessionId: string): void {
    if (sessions.value.has(sessionId)) {
      activeSessionId.value = sessionId;
    }
  }

  // ── Close session ─────────────────────────────────────────────────

  async function closeSession(sessionId: string): Promise<void> {
    const session = sessions.value.get(sessionId);
    if (!session) return;

    if (session.isConnected || session.isConnecting) {
      await disconnect(sessionId);
    } else {
      stopPermissionWatch(sessionId);
      bridges.delete(sessionId);
      sessions.value.delete(sessionId);
      adjustActiveSessionId(sessionId);
    }
  }

  // ── Delete saved session ──────────────────────────────────────────

  async function deleteSession(sessionId: string): Promise<void> {
    savedSessions.value = savedSessions.value.filter(s => s.id !== sessionId);
    await saveSessionsToStore();
  }

  // ── Set session mode ──────────────────────────────────────────────

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

  // ── Set session model ─────────────────────────────────────────────

  async function setModel(modelId: string): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) throw new Error('No active session');
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) {
      throw new Error('No active session');
    }

    await bridge.unstable_setSessionModel({
      sessionId: session.session.sessionId,
      modelId,
    });

    // Optimistically update the current model
    session.currentModelId = modelId;
  }

  // ── Set session config option ─────────────────────────────────────

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
      const errorSession = createEmptySessionState(localId, '', '');
      errorSession.error = msg;
      sessions.value.set(localId, errorSession);
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
    availableModels,
    currentModelId,
    startupPhase,
    startupLogs,
    startupElapsed,

    // Computed (derived)
    hasActiveSession,
    messageList,
    toolCallList,
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
