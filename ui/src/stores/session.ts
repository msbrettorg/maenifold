// Session store for managing ACP sessions and persistence
import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { loadKvStore, type KVStore } from '../lib/host/storage';
import { getAppVersion } from '../lib/host';
import { trackEvent, trackError } from '../lib/telemetry';
import type { SavedSession, ChatMessage, ToolCallInfo, PermissionRequest, SlashCommand, SessionConfigOption, AgentConfig } from '../lib/types';
import { getTransportKind } from '../lib/types';
import { AcpClientBridge, createAcpClient } from '../lib/acp-bridge';
import { onAgentStderr, spawnAgent, killAgent } from '../lib/host';
import { isDesktop } from '../lib/platform';
import { useConfigStore } from './config';
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

export const useSessionStore = defineStore('session', () => {
  // State
  const savedSessions = ref<SavedSession[]>([]);
  const currentSession = ref<SavedSession | null>(null);
  const messages = ref<ChatMessage[]>([]);
  const toolCalls = ref<Map<string, ToolCallInfo>>(new Map());
  const isConnected = ref(false);
  const isLoading = ref(false);
  const isConnecting = ref(false);
  // True while a foreground reconnect attempt is in flight. Distinct from
  // `isConnecting` (which is the multi-phase initial spawn/connect path):
  // reconnects skip the spawn/stderr-progress UI and just need a small
  // "Reconnecting…" indicator.
  const isReconnecting = ref(false);
  const error = ref<string | null>(null);
  const pendingPermission = ref<PermissionRequest | null>(null);
  
  // Authentication state
  const pendingAuthMethods = ref<AuthMethod[]>([]);
  const pendingAuthAgentName = ref<string>('');
  let authMethodResolver: ((methodId: string | null) => void) | null = null;
  
  // Session config options — canonical ACP surface (replaces the
  // deprecated session-modes + session-models pickers). Each option is a
  // self-describing selector the Agent advertises at session start and
  // updates via `config_option_update` notifications. See:
  //   https://agentclientprotocol.com/protocol/session-config-options
  const configOptions = ref<SessionConfigOption[]>([]);

  // Slash commands
  const availableCommands = ref<SlashCommand[]>([]);
  
  // Connection cancellation
  let connectionAborted = false;
  
  // Startup progress tracking
  const startupPhase = ref<string>('starting');
  const startupLogs = ref<string[]>([]);
  const startupElapsed = ref<number>(0);
  let startupTimer: ReturnType<typeof setInterval> | null = null;
  let stderrUnlisten: (() => void) | null = null;
  
  // Current ACP client
  let acpClient: AcpClientBridge | null = null;
  let store: KVStore | null = null;

  // Computed
  const hasActiveSession = computed(() => currentSession.value !== null);
  const messageList = computed(() => messages.value);
  const toolCallList = computed(() => Array.from(toolCalls.value.values()));
  // Only sessions that support resuming (loadSession capability)
  const resumableSessions = computed(() => 
    savedSessions.value.filter(s => s.supportsLoadSession === true)
  );

  // Initialize store
  async function initStore() {
    store = await loadKvStore(STORE_PATH);
    const saved = await store.get<SavedSession[]>('sessions');
    if (saved) {
      savedSessions.value = saved;
    }
    
    // Load app version (Tauri API on desktop/mobile, build-time inject on web)
    try {
      appVersion = await getAppVersion();
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

  // Handle an unexpected transport close (e.g. WebSocket dropped while idle,
  // local agent process exited). The bridge has already rejected any
  // in-flight requests; we just need to tear down UI state so the user gets
  // a clear "disconnected" signal instead of a stale "connected" view.
  function handleUnexpectedClose(reason?: string): void {
    // If `acpClient` is already null, this fired during a voluntary
    // disconnect that's tearing down anyway — nothing to do.
    if (!acpClient) return;
    acpClient = null;
    isConnected.value = false;
    isLoading.value = false;
    pendingPermission.value = null;
    error.value = `Connection lost: ${reason ?? 'transport closed'}`;
  }

  // Session update handler
  function handleSessionUpdate(notification: SessionNotification) {
    const update = notification.update;
    
    switch (update.sessionUpdate) {
      case 'user_message_chunk':
        // Append to last user message or create new (for replay)
        const lastUserMsg = messages.value[messages.value.length - 1];
        if (lastUserMsg && lastUserMsg.role === 'user') {
          if (update.content.type === 'text') {
            lastUserMsg.content += update.content.text;
          }
        } else {
          messages.value.push({
            id: crypto.randomUUID(),
            role: 'user',
            content: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
          });
        }
        break;

      case 'agent_message_chunk':
        // Append to last assistant message or create new
        const lastMsg = messages.value[messages.value.length - 1];
        if (lastMsg && lastMsg.role === 'assistant') {
          if (update.content.type === 'text') {
            lastMsg.content += update.content.text;
          }
        } else {
          messages.value.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
            toolCalls: [],
          });
        }
        break;

      case 'agent_thought_chunk':
        // Append to last assistant message's thought field or create new
        const lastAssistantMsg = messages.value[messages.value.length - 1];
        if (lastAssistantMsg && lastAssistantMsg.role === 'assistant') {
          if (update.content.type === 'text') {
            lastAssistantMsg.thought = (lastAssistantMsg.thought || '') + update.content.text;
          }
        } else {
          messages.value.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: '',
            thought: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
            toolCalls: [],
          });
        }
        break;

      case 'tool_call':
        // Add tool call to the current assistant message
        const currentAssistantMsg = messages.value[messages.value.length - 1];
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
        // Also keep in global map for updates
        toolCalls.value.set(update.toolCallId, {
          toolCallId: update.toolCallId,
          title: update.title,
          kind: update.kind || 'other',
          status: update.status || 'pending',
          locations: update.locations,
        });
        break;

      case 'tool_call_update':
        const existing = toolCalls.value.get(update.toolCallId);
        if (existing) {
          if (update.status) existing.status = update.status;
          if (update.title) existing.title = update.title;
          // Also update in the message's toolCalls array
          for (const msg of messages.value) {
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

      case 'current_mode_update':
        // Legacy ACP path — agents on configOptions emit `config_option_update`
        // instead. We still surface the change so a legacy agent (one that
        // hasn't migrated yet) gets its picker updated, by mapping the
        // modeId into the `category: "mode"` configOption when present.
        if ('modeId' in update && update.modeId) {
          const modeOpt = configOptions.value.find(o => o.category === 'mode' && o.type === 'select');
          if (modeOpt && modeOpt.type === 'select') {
            modeOpt.currentValue = update.modeId as string;
          }
        }
        break;

      case 'config_option_update':
        // Agent advertised the new authoritative configOptions snapshot.
        // Per spec the `configOptions` array IS the full state — replace
        // wholesale rather than merging.
        if ('configOptions' in update && Array.isArray(update.configOptions)) {
          configOptions.value = update.configOptions as unknown as SessionConfigOption[];
        }
        break;

      case 'available_commands_update':
        // Agent advertised slash commands
        if ('availableCommands' in update && Array.isArray(update.availableCommands)) {
          availableCommands.value = update.availableCommands.map((cmd) => ({
            name: cmd.name,
            description: cmd.description,
            hint: cmd.input?.hint ?? undefined,
          }));
        }
        break;

      default:
        console.log('Unhandled session update:', update);
    }
  }

  // Prompt user to select auth method
  async function promptForAuthMethod(authMethods: AuthMethod[], agentName: string): Promise<string | null> {
    return new Promise((resolve) => {
      pendingAuthMethods.value = authMethods;
      pendingAuthAgentName.value = agentName;
      authMethodResolver = resolve;
    });
  }

  // User selected an auth method
  function selectAuthMethod(methodId: string): void {
    if (authMethodResolver) {
      authMethodResolver(methodId);
      authMethodResolver = null;
      pendingAuthMethods.value = [];
      pendingAuthAgentName.value = '';
    }
  }

  // User cancelled auth selection
  function cancelAuthSelection(): void {
    if (authMethodResolver) {
      authMethodResolver(null);
      authMethodResolver = null;
      pendingAuthMethods.value = [];
      pendingAuthAgentName.value = '';
    }
  }

  // Create new session
  async function createSession(agentName: string, cwd: string): Promise<void> {
    isLoading.value = true;
    isConnecting.value = true;
    connectionAborted = false;
    error.value = null;

    // Look up the agent's transport kind so we know whether to do the
    // stdio-only startup choreography (spawn → stderr progress) or the
    // streamlined remote path (just open a network transport).
    const configStore = useConfigStore();
    const agentConfig: AgentConfig | undefined = configStore.getAgent(agentName);
    const transportKind = agentConfig
      ? getTransportKind(agentConfig)
      : 'stdio';
    const isRemote = transportKind !== 'stdio';

    // Reset and start progress tracking
    startupPhase.value = 'starting';
    startupLogs.value = [];
    startupElapsed.value = 0;
    startupTimer = setInterval(() => {
      startupElapsed.value++;
    }, 1000);

    // Track the spawned stdio instance separately so we can `killAgent` it
    // if cancellation/abort happens before we've wrapped it in a bridge.
    // Once `acpClient` is set, ownership transfers to the bridge and
    // `acpClient.disconnect()` becomes the only correct cleanup path.
    let spawnedInstance: { id: string } | null = null;

    try {
      if (!agentConfig) {
        throw new Error(`Agent '${agentName}' not found in config`);
      }

      if (!isRemote) {
        // For stdio agents we need the spawned process's id up front so the
        // stderr listener can filter on it (multiple agents may be running
        // concurrently). We spawn here, hand the resulting AgentInstance to
        // a StdioTransport, then build the bridge from that transport.
        startupPhase.value = 'starting';
        const agentInstance = await spawnAgent(agentName);
        spawnedInstance = agentInstance;

        stderrUnlisten = await onAgentStderr((stderr) => {
          if (stderr.agent_id !== agentInstance.id) return;
          startupLogs.value.push(stderr.line);
          // Detect phase from output
          const detectedPhase = detectPhase(stderr.line);
          if (detectedPhase) {
            startupPhase.value = detectedPhase;
          }
        }) as unknown as () => void;

        if (connectionAborted) {
          // Process was spawned but no bridge exists yet — kill the orphan
          // before throwing so the local agent doesn't keep running.
          await killAgent(agentInstance.id).catch((err) =>
            console.warn('killAgent during abort failed:', err)
          );
          spawnedInstance = null;
          throw new Error('Connection cancelled');
        }

        startupPhase.value = 'initializing';

        // Wrap the just-spawned instance in a StdioTransport. Using the
        // legacy single-arg form keeps backward compatibility and avoids a
        // double-spawn (StdioTransport.spawn would call spawnAgent again).
        acpClient = await createAcpClient(agentInstance);
        // Ownership of the child process now belongs to the bridge — clear
        // our local reference so the catch block doesn't double-kill it.
        spawnedInstance = null;
      } else {
        // Remote agents have no stderr stream; show a minimal "connecting"
        // state instead of the multi-phase progress UI.
        startupPhase.value = 'connecting';

        if (connectionAborted) {
          throw new Error('Connection cancelled');
        }

        // The factory opens a WebSocket / HTTP connection based on
        // agentConfig.transport.
        acpClient = await createAcpClient({ name: agentName, config: agentConfig });
      }

      acpClient.onSessionUpdate = handleSessionUpdate;
      // Surface unexpected transport closes (e.g. WebSocket drop while idle)
      // to the UI so users don't sit on a stale "connected" state forever.
      acpClient.onTransportClose = (reason) => {
        handleUnexpectedClose(reason);
      };
      
      // Sync bridge's pendingPermissionRequest to store's pendingPermission
      watch(
        () => acpClient?.pendingPermissionRequest.value,
        (newValue) => {
          pendingPermission.value = newValue ?? null;
        },
        { immediate: true }
      );

      if (connectionAborted) {
        await acpClient.disconnect();
        throw new Error('Connection cancelled');
      }

      startupPhase.value = 'connecting';

      // Initialize connection
      // Only Tauri desktop has real filesystem access; mobile and web
      // cannot fulfil readTextFile / writeTextFile RPCs.
      const canAccessFs = isDesktop();

      const initResponse = await acpClient.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: {
          fs: {
            readTextFile: canAccessFs,
            writeTextFile: canAccessFs,
          },
        },
        clientInfo: {
          name: 'maenifold-ui',
          title: 'Maenifold',
          version: appVersion,
        },
      });

      console.log('Agent initialized:', initResponse);

      // Check if agent supports session loading
      const supportsLoadSession = initResponse.agentCapabilities?.loadSession ?? false;

      if (connectionAborted) {
        await acpClient.disconnect();
        throw new Error('Connection cancelled');
      }

      // Store available auth methods for potential retry
      const availableAuthMethods = initResponse.authMethods || [];

      if (connectionAborted) {
        await acpClient.disconnect();
        throw new Error('Connection cancelled');
      }

      // Try to create session - may fail with auth_required
      let sessionResponse;
      try {
        sessionResponse = await acpClient.newSession({
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
          const selectedMethodId = await promptForAuthMethod(availableAuthMethods, agentName);
          
          if (!selectedMethodId || connectionAborted) {
            await acpClient.disconnect();
            throw new Error('Authentication cancelled by user');
          }
          
          console.log('Authenticating with method:', selectedMethodId);
          const authResponse = await acpClient.authenticate({
            methodId: selectedMethodId,
          });
          console.log('Authentication successful:', authResponse);

          if (connectionAborted) {
            await acpClient.disconnect();
            throw new Error('Connection cancelled');
          }

          // Retry session creation after auth
          sessionResponse = await acpClient.newSession({
            cwd,
            mcpServers: [],
          });
        } else {
          throw sessionError;
        }
      }

      // Save session
      const session: SavedSession = {
        id: crypto.randomUUID(),
        agentName,
        sessionId: sessionResponse.sessionId,
        title: `Session ${new Date().toLocaleString()}`,
        lastUpdated: Date.now(),
        cwd,
        supportsLoadSession,
      };

      currentSession.value = session;
      savedSessions.value.push(session);
      await saveSessionsToStore();
      
      isConnected.value = true;
      messages.value = [];
      toolCalls.value.clear();
      
      // Track successful session creation
      trackEvent('SessionCreated', { agentName, success: 'true' });
      
      // Hydrate session config options from the new ACP surface. Per spec
      // (https://agentclientprotocol.com/protocol/session-config-options)
      // `configOptions` is the canonical replacement for `modes` and
      // `models`; Agents that still emit those legacy fields will also
      // mirror them under `configOptions`. We only consume the latter.
      const respConfigOptions = (sessionResponse as unknown as { configOptions?: unknown[] }).configOptions;
      if (Array.isArray(respConfigOptions)) {
        configOptions.value = respConfigOptions as SessionConfigOption[];
      } else {
        configOptions.value = [];
      }

    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      // Tear down whichever side of the connection is live. The bridge owns
      // the spawned process once it exists, so prefer disconnecting it.
      // Otherwise (e.g. abort right after spawn but before bridge creation)
      // kill the orphaned local agent directly.
      if (acpClient) {
        try {
          await acpClient.disconnect();
        } catch (cleanupErr) {
          console.warn('disconnect during createSession cleanup failed:', cleanupErr);
        }
      } else if (spawnedInstance) {
        try {
          await killAgent(spawnedInstance.id);
        } catch (cleanupErr) {
          console.warn('killAgent during createSession cleanup failed:', cleanupErr);
        }
      }
      acpClient = null;
      // Track session creation failure
      trackEvent('SessionCreated', { agentName, success: 'false' });
      trackError(e instanceof Error ? e : new Error(String(e)));
      throw e;
    } finally {
      isLoading.value = false;
      isConnecting.value = false;
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

  // Resume existing session
  async function resumeSession(savedSession: SavedSession): Promise<void> {
    isLoading.value = true;
    error.value = null;

    // Tear down any stale acpClient before standing up a new one.
    // Without this, double-clicking a saved session (or resuming after a
    // dropped connection) leaks the prior subprocess and races two
    // initialize() requests on different transports.
    if (acpClient) {
      try {
        await acpClient.disconnect();
      } catch (cleanupErr) {
        console.warn('disconnect of stale client during resume failed:', cleanupErr);
      }
      acpClient = null;
    }
    // Reset replay buffers up-front so the agent's session/load replay
    // streams into a clean slate instead of duplicating history.
    messages.value = [];
    toolCalls.value.clear();
    configOptions.value = [];
    availableCommands.value = [];

    const configStore = useConfigStore();
    const agentConfig: AgentConfig | undefined = configStore.getAgent(savedSession.agentName);
    const transportKind = agentConfig
      ? getTransportKind(agentConfig)
      : 'stdio';
    const isRemote = transportKind !== 'stdio';

    // Stdio agents emit startup diagnostics on stderr (npm download
    // progress, agent boot errors, OAuth prompts, etc.). createSession
    // already captures them; resumeSession used to ignore them and
    // surface a useless "transport closed: agent process exited" when
    // the agent died at startup. Capture them here too so the user gets
    // an actionable error.
    let spawnedInstance: { id: string } | null = null;

    try {
      if (!agentConfig) {
        throw new Error(`Agent '${savedSession.agentName}' not found in config`);
      }

      if (!isRemote) {
        const agentInstance = await spawnAgent(savedSession.agentName);
        spawnedInstance = agentInstance;

        stderrUnlisten = await onAgentStderr((stderr) => {
          if (stderr.agent_id !== agentInstance.id) return;
          startupLogs.value.push(stderr.line);
        }) as unknown as () => void;

        acpClient = await createAcpClient(agentInstance);
        spawnedInstance = null;
      } else {
        acpClient = await createAcpClient({
          name: savedSession.agentName,
          config: agentConfig,
        });
      }

      acpClient.onSessionUpdate = handleSessionUpdate;
      acpClient.onTransportClose = (reason) => {
        handleUnexpectedClose(reason);
      };

      watch(
        () => acpClient?.pendingPermissionRequest.value,
        (newValue) => {
          pendingPermission.value = newValue ?? null;
        },
        { immediate: true }
      );

      // Only Tauri desktop has real filesystem access; mobile and web
      // cannot fulfil readTextFile / writeTextFile RPCs.
      const canAccessFs = isDesktop();

      // Initialize connection
      const initResponse = await acpClient.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: {
          fs: {
            readTextFile: canAccessFs,
            writeTextFile: canAccessFs,
          },
        },
        clientInfo: {
          name: 'maenifold-ui',
          title: 'Maenifold',
          version: appVersion,
        },
      });

      // Store available auth methods for potential retry
      const availableAuthMethods = initResponse.authMethods || [];

      // Try to load existing session — may fail with auth_required.
      // The agent replays the conversation via session/update notifications
      // during this call; handleSessionUpdate above appends them to
      // `messages` so the history is visible by the time we return.
      let loadResponse;
      try {
        loadResponse = await acpClient.loadSession({
          sessionId: savedSession.sessionId,
          cwd: savedSession.cwd,
          mcpServers: [],
        });
      } catch (sessionError: unknown) {
        const errorMessage = sessionError instanceof Error ? sessionError.message : String(sessionError);
        const isAuthRequired = errorMessage.toLowerCase().includes('authentication required') ||
                               errorMessage.includes('-32000');

        if (isAuthRequired && availableAuthMethods.length > 0) {
          console.log('Authentication required, available methods:', availableAuthMethods);

          const selectedMethodId = await promptForAuthMethod(availableAuthMethods, savedSession.agentName);

          if (!selectedMethodId) {
            await acpClient.disconnect();
            throw new Error('Authentication cancelled by user');
          }

          console.log('Authenticating with method:', selectedMethodId);
          const authResponse = await acpClient.authenticate({
            methodId: selectedMethodId,
          });
          console.log('Authentication successful:', authResponse);

          // Retry loading session after auth
          loadResponse = await acpClient.loadSession({
            sessionId: savedSession.sessionId,
            cwd: savedSession.cwd,
            mcpServers: [],
          });
        } else {
          throw sessionError;
        }
      }

      // Hydrate session config options from the LoadSessionResponse. The
      // resume path used to discard this — leaving the top-right picker
      // row empty after every resume even when the agent returned the
      // canonical configOptions snapshot. See:
      //   https://agentclientprotocol.com/protocol/session-config-options
      const respConfigOptions = (loadResponse as unknown as { configOptions?: unknown[] }).configOptions;
      if (Array.isArray(respConfigOptions)) {
        configOptions.value = respConfigOptions as SessionConfigOption[];
      }

      currentSession.value = savedSession;
      isConnected.value = true;

      // Track successful session resume
      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'true' });

      // Update last accessed time
      savedSession.lastUpdated = Date.now();
      await saveSessionsToStore();

    } catch (e) {
      // Detect "session not found" responses from the agent. The saved
      // sessionId persists in localStorage across UI launches, but many
      // ACP agents (e.g. GitHub Copilot) keep session state in-process
      // only — so a saved session from a prior agent run is permanently
      // dead. Remove it from the saved list so it stops cluttering the
      // resume picker, and surface an actionable message instead of the
      // raw JSON-RPC error.
      const baseMessage = e instanceof Error ? e.message : String(e);
      const isStaleSession = /resource not found|session.*not found|-32002/i.test(baseMessage);
      if (isStaleSession) {
        try {
          await deleteSession(savedSession.id);
        } catch (delErr) {
          console.warn('deleteSession after stale-resume cleanup failed:', delErr);
        }
      }

      // Stitch any captured stderr into the error so the user sees what
      // actually broke instead of just "transport closed".
      const stderrTail = startupLogs.value.slice(-5).filter((l) => l.trim().length > 0);
      const composedMessage = isStaleSession
        ? `Session is no longer available on the agent (it was pruned). Removed from saved sessions — start a new one.`
        : (stderrTail.length > 0
            ? `${baseMessage}\nAgent stderr (last ${stderrTail.length} lines):\n${stderrTail.join('\n')}`
            : baseMessage);
      error.value = composedMessage;

      // Disconnect the bridge if it was created — otherwise we leak the
      // spawned stdio process or open WebSocket on initialize/loadSession
      // failure.
      if (acpClient) {
        try {
          await acpClient.disconnect();
        } catch (cleanupErr) {
          console.warn('disconnect during resumeSession cleanup failed:', cleanupErr);
        }
        acpClient = null;
      } else if (spawnedInstance) {
        try {
          await killAgent(spawnedInstance.id);
        } catch (cleanupErr) {
          console.warn('killAgent during resumeSession cleanup failed:', cleanupErr);
        }
      }
      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'false' });
      trackError(e instanceof Error ? e : new Error(String(e)));
      throw e;
    } finally {
      isLoading.value = false;
      if (stderrUnlisten) {
        stderrUnlisten();
        stderrUnlisten = null;
      }
    }
  }

  // Send prompt
  async function sendPrompt(text: string): Promise<void> {
    if (!acpClient || !currentSession.value) {
      throw new Error('No active session');
    }

    // Add user message
    messages.value.push({
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    });

    isLoading.value = true;
    try {
      const response = await acpClient.prompt({
        sessionId: currentSession.value.sessionId,
        prompt: [
          {
            type: 'text',
            text,
          },
        ],
      });

      console.log('Prompt completed:', response.stopReason);

      // Track prompt sent
      trackEvent('PromptSent', { 
        messageLength: String(text.length),
        stopReason: response.stopReason || 'unknown',
      });

      // Update session title if it's the first message
      if (messages.value.length === 2 && currentSession.value) {
        currentSession.value.title = text.slice(0, 50) + (text.length > 50 ? '...' : '');
        currentSession.value.lastUpdated = Date.now();
        await saveSessionsToStore();
      }
    } finally {
      isLoading.value = false;
    }
  }

  // Cancel current operation
  async function cancelOperation(): Promise<void> {
    if (!acpClient || !currentSession.value) return;
    
    await acpClient.cancel({
      sessionId: currentSession.value.sessionId,
    });
  }

  // Cancel ongoing connection attempt
  async function cancelConnection(): Promise<void> {
    connectionAborted = true;
    
    // Cancel auth selection if pending
    if (authMethodResolver) {
      authMethodResolver(null);
      authMethodResolver = null;
      pendingAuthMethods.value = [];
      pendingAuthAgentName.value = '';
    }
    
    // Disconnect if client exists
    if (acpClient) {
      try {
        await acpClient.disconnect();
      } catch (e) {
        console.error('Error disconnecting:', e);
      }
      acpClient = null;
    }
    
    isLoading.value = false;
    isConnecting.value = false;
    error.value = null;
  }

  // Handle permission response
  function resolvePermission(optionId: string): void {
    if (acpClient) {
      acpClient.resolvePermission(optionId);
    }
  }

  function cancelPermission(): void {
    if (acpClient) {
      acpClient.cancelPermission();
    }
  }

  // Disconnect current session
  async function disconnect(): Promise<void> {
    const agentName = currentSession.value?.agentName || 'unknown';
    const sessionStart = currentSession.value?.lastUpdated || Date.now();
    const sessionDuration = Math.round((Date.now() - sessionStart) / 1000);
    
    if (acpClient) {
      await acpClient.disconnect();
      acpClient = null;
    }
    
    // Track session disconnect
    trackEvent('SessionDisconnected', { 
      agentName,
      sessionDurationSeconds: String(sessionDuration),
      messageCount: String(messages.value.length),
    });
    
    currentSession.value = null;
    isConnected.value = false;
    messages.value = [];
    toolCalls.value.clear();
    availableCommands.value = [];
    configOptions.value = [];
  }

  // Delete saved session
  async function deleteSession(sessionId: string): Promise<void> {
    savedSessions.value = savedSessions.value.filter(s => s.id !== sessionId);
    await saveSessionsToStore();
  }

  /**
   * Change a session configuration option (the canonical ACP surface that
   * replaced session/set_mode + session/set_model). The Agent returns the
   * full updated `configOptions` snapshot; we replace local state with that
   * authoritative response rather than guessing optimistically.
   *
   * `value` is `string` for `type: "select"` options or `boolean` for
   * `type: "boolean"` options.
   */
  async function setConfigOption(configId: string, value: string | boolean): Promise<void> {
    if (!acpClient || !currentSession.value) {
      throw new Error('No active session');
    }

    const response = await acpClient.setConfigOption({
      sessionId: currentSession.value.sessionId,
      configId,
      value,
    });

    if (response && Array.isArray(response.configOptions)) {
      configOptions.value = response.configOptions as unknown as SessionConfigOption[];
    } else {
      // Fall back to optimistic update if the agent's response shape is
      // incomplete — keep the UI responsive but log so we can debug.
      console.warn('setConfigOption: agent response missing configOptions, applying optimistic update');
      const opt = configOptions.value.find((o) => o.id === configId);
      if (opt) {
        // Reassign through a typed lane so TS keeps the discriminated union sound.
        if (opt.type === 'select' && typeof value === 'string') {
          opt.currentValue = value;
        } else if (opt.type === 'boolean' && typeof value === 'boolean') {
          opt.currentValue = value;
        }
      }
    }
  }

  function clearError() {
    error.value = null;
  }

  /**
   * Foreground reconnect: when the user returns to the app and we're
   * disconnected (because the OS froze the WebView, the NAT killed the TCP
   * connection, or the network changed), silently re-attach to the saved
   * session if possible.
   *
   * Returns `true` if a reconnect was attempted, `false` if there was
   * nothing to do (no saved session, already connected/connecting, agent
   * doesn't advertise session-load support, etc.).
   *
   * Errors are surfaced via `error.value` exactly like a manual resume
   * would; the caller doesn't need to handle them.
   */
  async function tryReconnect(): Promise<boolean> {
    // Already connected or already trying — leave it alone.
    if (isConnected.value || isConnecting.value || isLoading.value) {
      return false;
    }
    // No prior session to reconnect to.
    const session = currentSession.value;
    if (!session) {
      return false;
    }
    // Bridge already exists (race with another reconnect in flight).
    if (acpClient) {
      return false;
    }
    // Agent must support `session/load` for resume to be meaningful;
    // otherwise we'd just create a fresh session, which is a strictly
    // user-initiated action.
    if (!session.supportsLoadSession) {
      return false;
    }

    // Clear the stale "Connection lost" banner up-front so the UI shows
    // an honest "Reconnecting…" state instead of a contradictory red
    // banner during the attempt. If the reconnect ultimately fails, the
    // catch below restores a real error message.
    error.value = null;
    isReconnecting.value = true;
    try {
      await resumeSession(session);
      return true;
    } catch (e) {
      // `resumeSession`'s own catch already wrote `error.value`; nothing
      // more to do here. Returning true so the caller knows we tried.
      console.warn('Foreground reconnect failed:', e);
      return true;
    } finally {
      isReconnecting.value = false;
    }
  }

  return {
    // State
    savedSessions,
    currentSession,
    messages,
    isConnected,
    isLoading,
    isConnecting,
    isReconnecting,
    error,
    pendingPermission,
    pendingAuthMethods,
    pendingAuthAgentName,
    configOptions,
    availableCommands,
    startupPhase,
    startupLogs,
    startupElapsed,
    
    // Computed
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
    setConfigOption,
    clearError,
    tryReconnect,
    
    // Expose client for permission handling
    get acpClient() { return acpClient; },
  };
});
