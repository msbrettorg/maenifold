import { ref, computed, onMounted, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { load, type Store } from '@tauri-apps/plugin-store';
import { isAbsolutePath } from '../lib/acp-meta';
import { PREFS_STORE_PATH } from '../lib/constants';
import type { AppAuthMethod } from '../lib/auth-methods';
import { initTelemetry } from '../lib/telemetry';
import type { AgentsConfig, PermissionRequest, SavedSession, SessionState } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';

export type ConfigStoreLike = {
  error: string | null; hasAgents: boolean; loadConfig(): Promise<void>;
  config: AgentsConfig;
  agentNames: string[];
  setupHotReload(): Promise<void>; clearError(): void;
};

export type SessionStoreLike = {
  isConnected: boolean; isConnecting: boolean; error: string | null;
  pendingPermission: PermissionRequest | null; pendingPermissionCount: number;
  pendingAuthMethods: AppAuthMethod[]; pendingAuthAgentName: string;
  sessions: Map<string, SessionState>; initStore(): Promise<void>;
  createSession(agentName: string, cwd: string): Promise<void>;
  resumeSession(session: SavedSession): Promise<void>;
  deleteSession(sessionId: string): Promise<void>; cancelConnection(): Promise<void>;
  resolvePermission(optionId: string): void; cancelPermission(): void;
  selectAuthMethod(methodId: string): void; cancelAuthSelection(): void;
  setActiveSession(localId: string): void; setError(message: string): void; clearError(): void;
};

export function useAppSession({
  configStore,
  sessionStore,
}: { configStore: ConfigStoreLike; sessionStore: SessionStoreLike }) {
  const selectedAgent = ref('');
  const selectedCwd = ref('');
  const agentCapabilitiesStore = useAgentCapabilitiesStore();
  let agentProbeBootstrapped = false;

  let prefsStore: Store | null = null;

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
      if (agentProbeBootstrapped) {
        agentCapabilitiesStore.invalidateAll();
      }
    },
    { deep: true },
  );

  // Bootstrap: load preferences, init telemetry, load config, init session store
  onMounted(async () => {
    try {
      prefsStore = await load(PREFS_STORE_PATH);
      const telemetryEnabled = await prefsStore.get<boolean>('telemetryEnabled') ?? true;
      await initTelemetry(telemetryEnabled);
      await configStore.loadConfig();
      await configStore.setupHotReload();
      agentProbeBootstrapped = true;
      void agentCapabilitiesStore.probeAll();
      await sessionStore.initStore();

      const savedCwd = await prefsStore.get<string>('lastCwd');
      if (savedCwd) {
        selectedCwd.value = savedCwd;
      }
    } catch {
      console.warn('Tauri APIs unavailable, running in browser mode');
    }
  });

  async function handleAgentSelect(agentName: string) {
    selectedAgent.value = agentName;
  }

  async function handleSelectFolder() {
    const folder = await open({ directory: true, multiple: false, title: 'Select Working Directory' });
    if (folder) {
      selectedCwd.value = folder as string;
      if (prefsStore) await prefsStore.set('lastCwd', folder);
    }
  }

  async function handleNewSession() {
    if (!selectedAgent.value) return;
    const cwd = selectedCwd.value.trim();
    if (!cwd) return sessionStore.setError('Working directory is required');
    if (!isAbsolutePath(cwd)) return sessionStore.setError('Working directory must be an absolute path');

    try {
      await sessionStore.createSession(selectedAgent.value, cwd);
    } catch (e) {
      console.error('Failed to create session:', e);
    }
  }

  async function handleResumeSession(session: SavedSession) {
    selectedAgent.value = session.agentName;
    try {
      for (const [localId, liveSession] of sessionStore.sessions.entries()) {
        if (liveSession.session?.sessionId === session.sessionId) {
          sessionStore.setActiveSession(localId);
          return;
        }
      }
      await sessionStore.resumeSession(session);
    } catch (e) {
      console.error('Failed to resume session:', e);
    }
  }

  async function handleDeleteSession(sessionId: string) {
    await sessionStore.deleteSession(sessionId);
  }
  async function handleCancelConnection() { await sessionStore.cancelConnection(); }

  function handlePermissionSelect(optionId: string) { sessionStore.resolvePermission(optionId); }
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
