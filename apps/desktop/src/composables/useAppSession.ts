import { computed, onMounted, ref, watch } from 'vue';
import type { AppAuthMethod } from '../lib/auth-methods';
import { isAbsolutePath } from '../lib/acp-meta';
import { PREFS_STORE_PATH } from '../lib/constants';
import { initTelemetry } from '../lib/telemetry';
import { pickAbsoluteDirectory } from './folder-picker';
import type { AgentsConfig, PermissionRequest, SavedSession, SessionSource, SessionState } from '../lib/types';
import type { PermissionScope } from '../lib/permission-ledger/ledger';
import {
  resolvePreferredAgent,
  writeLastUsedAgent,
} from '../lib/preferred-agent';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useAgentSessionOverridesStore } from '../stores/agent-session-overrides';

export type ConfigStoreLike = {
  error: string | null; hasAgents: boolean; loadConfig(): Promise<void>;
  config: AgentsConfig;
  agentNames: string[];
  setupHotReload(): Promise<void>; clearError(): void;
};

export type SessionStoreLike = {
  isConnected: boolean; isConnecting: boolean; error: string | null;
  activeSessionId: string | null;
  pendingPermission: PermissionRequest | null; pendingPermissionCount: number;
  pendingAuthMethods: AppAuthMethod[]; pendingAuthAgentName: string;
  sessions: Map<string, SessionState>; initStore(): Promise<void>;
  createSession(agentName: string, cwd: string, configOptions?: Record<string, string>, options?: { source?: SessionSource }): Promise<void>;
  resumeSession(session: SavedSession): Promise<void>;
  deleteSession(sessionId: string): Promise<void>; cancelConnection(): Promise<void>;
  resolvePermission(optionId: string, scope?: PermissionScope): void; cancelPermission(): void;
  selectAuthMethod(methodId: string): void; cancelAuthSelection(): void;
  setActiveSession(localId: string): void; setError(message: string): void; clearError(): void;
};

type BrowserPrefsStore = {
  get<T>(key: string): Promise<T | null>;
  set(key: string, value: unknown): Promise<void>;
};

function createPrefsStore(): BrowserPrefsStore {
  return {
    async get<T>(key: string) {
      if (typeof window === 'undefined') {
        return null;
      }

      const raw = window.localStorage.getItem(`${PREFS_STORE_PATH}:${key}`);
      if (raw === null) {
        return null;
      }

      try {
        return JSON.parse(raw) as T;
      } catch {
        return raw as T;
      }
    },
    async set(key: string, value: unknown) {
      if (typeof window === 'undefined') {
        return;
      }

      window.localStorage.setItem(`${PREFS_STORE_PATH}:${key}`, JSON.stringify(value));
    },
  };
}

export function useAppSession({
  configStore,
  sessionStore,
}: { configStore: ConfigStoreLike; sessionStore: SessionStoreLike }) {
  const selectedAgent = ref('');
  const selectedCwd = ref('');
  const agentCapabilitiesStore = useAgentCapabilitiesStore();
  const sessionOverridesStore = useAgentSessionOverridesStore();
  let agentProbeBootstrapped = false;

  const prefsStore = createPrefsStore();

  const isConnected = computed(() => sessionStore.isConnected);
  const isConnecting = computed(() => sessionStore.isConnecting);
  const error = computed(() => sessionStore.error || configStore.error);
  const hasAgents = computed(() => configStore.hasAgents);
  const pendingPermission = computed(() => sessionStore.pendingPermission);
  const pendingPermissionCount = computed(() => sessionStore.pendingPermissionCount);
  const pendingAuthMethods = computed<AppAuthMethod[]>(
    () => sessionStore.pendingAuthMethods
  );
  const pendingAuthAgentName = computed(() => sessionStore.pendingAuthAgentName);

  watch(
    () => configStore.config,
    () => {
      const preferredAgent = resolvePreferredAgent(configStore.agentNames);
      if (!preferredAgent) {
        selectedAgent.value = '';
      } else if (!configStore.agentNames.includes(selectedAgent.value)) {
        selectedAgent.value = preferredAgent;
        writeLastUsedAgent(preferredAgent);
      }

      if (agentProbeBootstrapped) {
        agentCapabilitiesStore.invalidateAll();
      }
    },
    { deep: true },
  );

  // Clear session overrides from the previously-selected agent whenever the
  // active agent changes (manual pick, config reload, or session resume).
  watch(selectedAgent, (newAgent, oldAgent) => {
    if (oldAgent && oldAgent !== newAgent) {
      sessionOverridesStore.clearSelectedOverrides(oldAgent);
    }
  });

  onMounted(async () => {
    const telemetryEnabled = await prefsStore.get<boolean>('telemetryEnabled') ?? false;
    await initTelemetry(telemetryEnabled);
    await configStore.loadConfig();
    await configStore.setupHotReload();
    const preferredAgent = resolvePreferredAgent(configStore.agentNames);
    if (preferredAgent) {
      selectedAgent.value = preferredAgent;
      writeLastUsedAgent(preferredAgent);
    }
    agentProbeBootstrapped = true;
    void agentCapabilitiesStore.probeAll();
    await sessionStore.initStore();

    const savedCwd = await prefsStore.get<string>('lastCwd');
    if (savedCwd) {
      selectedCwd.value = savedCwd;
    }
  });

  async function handleAgentSelect(agentName: string) {
    selectedAgent.value = agentName;
    writeLastUsedAgent(agentName);
  }

  async function handleSelectFolder() {
    const selectionResult = await pickAbsoluteDirectory(selectedCwd.value);
    if (selectionResult.status === 'cancelled') return;
    if (selectionResult.status === 'error') {
      sessionStore.setError(selectionResult.message || 'Failed to select working directory');
      return;
    }
    if (selectionResult.status !== 'picked') return;

    const folder = selectionResult.path.trim();
    if (!isAbsolutePath(folder)) {
      sessionStore.setError('Working directory must be an absolute path');
      return;
    }

    selectedCwd.value = folder;
    await prefsStore.set('lastCwd', folder);
  }

  async function handleNewSession() {
    if (!selectedAgent.value) return;
    const cwd = selectedCwd.value.trim();
    if (!cwd) return sessionStore.setError('Working directory is required');
    if (!isAbsolutePath(cwd)) return sessionStore.setError('Working directory must be an absolute path');

    try {
      await sessionStore.createSession(
        selectedAgent.value,
        cwd,
        sessionOverridesStore.getSelectedOverrides(selectedAgent.value),
      );
      sessionOverridesStore.clearSelectedOverrides(selectedAgent.value);
    } catch (e) {
      console.error('Failed to create session:', e);
    }
  }

  async function handleResumeSession(session: SavedSession) {
    try {
      for (const [localId, liveSession] of sessionStore.sessions.entries()) {
        if (liveSession.session?.sessionId === session.sessionId && liveSession.isConnected) {
          sessionStore.setActiveSession(localId);
          selectedAgent.value = session.agentName;
          writeLastUsedAgent(session.agentName);
          return;
        }
      }
      await sessionStore.resumeSession(session);
      selectedAgent.value = session.agentName;
      writeLastUsedAgent(session.agentName);
    } catch (e) {
      console.error('Failed to resume session:', e);
    }
  }

  async function handleDeleteSession(sessionId: string) {
    await sessionStore.deleteSession(sessionId);
  }
  async function handleCancelConnection() { await sessionStore.cancelConnection(); }

  function handlePermissionSelect(optionId: string, scope?: PermissionScope) {
    sessionStore.resolvePermission(optionId, scope);
  }
  function handlePermissionCancel() { sessionStore.cancelPermission(); }
  function handleAuthMethodSelect(methodId: string) { sessionStore.selectAuthMethod(methodId); }
  function handleAuthMethodCancel() { sessionStore.cancelAuthSelection(); }

  return {
    selectedAgent,
    selectedCwd,
    isConnected,
    isConnecting,
    error,
    hasAgents,
    pendingPermission,
    pendingPermissionCount,
    pendingAuthMethods,
    pendingAuthAgentName,
    handleAgentSelect,
    handleSelectFolder,
    handleNewSession,
    handleResumeSession,
    handleDeleteSession,
    handleCancelConnection,
    handlePermissionSelect,
    handlePermissionCancel,
    handleAuthMethodSelect,
    handleAuthMethodCancel,
  };
}
