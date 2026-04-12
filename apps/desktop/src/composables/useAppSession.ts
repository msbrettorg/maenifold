import { computed, onMounted, ref, watch } from 'vue';
import type { AppAuthMethod } from '../lib/auth-methods';
import { isAbsolutePath } from '../lib/acp-meta';
import { PREFS_STORE_PATH } from '../lib/constants';
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

async function promptForAbsolutePath(title: string, initialValue = ''): Promise<string> {
  if (typeof document === 'undefined') {
    return '';
  }

  if (typeof HTMLDialogElement === 'undefined') {
    return (window.prompt(title, initialValue) ?? '').trim();
  }

  return new Promise((resolve) => {
    const dialog = document.createElement('dialog');
    dialog.className = 'native-path-dialog';
    const form = document.createElement('form');
    form.method = 'dialog';
    form.className = 'native-path-dialog__form';

    const heading = document.createElement('h3');
    heading.textContent = title;
    heading.className = 'native-path-dialog__title';

    const input = document.createElement('input');
    input.type = 'text';
    input.placeholder = '/absolute/path';
    input.value = initialValue;
    input.autofocus = true;
    input.className = 'native-path-dialog__input';

    const actions = document.createElement('div');
    actions.className = 'native-path-dialog__actions';

    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.textContent = 'Cancel';
    cancel.className = 'native-path-dialog__button native-path-dialog__button--cancel';
    cancel.addEventListener('click', () => dialog.close('cancel'));

    const submit = document.createElement('button');
    submit.type = 'submit';
    submit.textContent = 'Use Path';
    submit.className = 'native-path-dialog__button native-path-dialog__button--submit';

    actions.append(cancel, submit);
    form.append(heading, input, actions);
    dialog.append(form);
    document.body.append(dialog);

    dialog.addEventListener('close', () => {
      const value = dialog.returnValue === 'cancel' ? '' : input.value.trim();
      dialog.remove();
      resolve(value);
    }, { once: true });

    dialog.showModal();
  });
}

export function useAppSession({
  configStore,
  sessionStore,
}: { configStore: ConfigStoreLike; sessionStore: SessionStoreLike }) {
  const selectedAgent = ref('');
  const selectedCwd = ref('');
  const agentCapabilitiesStore = useAgentCapabilitiesStore();
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
      if (agentProbeBootstrapped) {
        agentCapabilitiesStore.invalidateAll();
      }
    },
    { deep: true },
  );

  onMounted(async () => {
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
  });

  async function handleAgentSelect(agentName: string) {
    selectedAgent.value = agentName;
  }

  async function handleSelectFolder() {
    const folder = await promptForAbsolutePath('Enter Working Directory', selectedCwd.value);
    if (!folder) {
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
