/* @vitest-environment jsdom */

import { flushPromises, mount } from '@vue/test-utils';
import { defineComponent, nextTick } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { SavedSession } from './lib/types';

const {
  createSessionMock,
  resumeSessionMock,
  setActiveTabMock,
  loadConfigMock,
  setupHotReloadMock,
  initStoreMock,
  loadKvStoreMock,
  initTelemetryMock,
  resumeTarget,
} = vi.hoisted(() => ({
  createSessionMock: vi.fn(),
  resumeSessionMock: vi.fn(),
  setActiveTabMock: vi.fn(),
  loadConfigMock: vi.fn(),
  setupHotReloadMock: vi.fn(),
  initStoreMock: vi.fn(),
  loadKvStoreMock: vi.fn(),
  initTelemetryMock: vi.fn(),
  resumeTarget: {
    id: 'saved-1',
    agentName: 'planner',
    sessionId: 'session-1',
    title: 'Saved session',
    lastUpdated: Date.now(),
    cwd: 'C:\\saved',
    supportsLoadSession: true,
  } satisfies SavedSession,
}));

const stores = vi.hoisted(() => ({
  sessionStore: undefined as any,
  shellStore: undefined as any,
}));

vi.mock('./stores/session', async () => {
  const { reactive } = await import('vue');

  stores.sessionStore = reactive({
    savedSessions: [] as SavedSession[],
    resumableSessions: [resumeTarget] as SavedSession[],
    currentSession: null as SavedSession | null,
    messages: [],
    toolCalls: new Map<string, unknown>(),
    isConnected: false,
    isLoading: false,
    isConnecting: false,
    isReconnecting: false,
    error: null as string | null,
    pendingPermission: null,
    pendingAuthMethods: [] as unknown[],
    pendingAuthAgentName: '',
    startupPhase: 'starting',
    startupLogs: [] as string[],
    startupElapsed: 0,
    initStore: initStoreMock,
    createSession: createSessionMock,
    resumeSession: resumeSessionMock,
    deleteSession: vi.fn(),
    disconnect: vi.fn(),
    cancelConnection: vi.fn(),
    clearError() {
      stores.sessionStore.error = null;
    },
    tryReconnect: vi.fn(),
    resolvePermission: vi.fn(),
    cancelPermission: vi.fn(),
    selectAuthMethod: vi.fn(),
    cancelAuthSelection: vi.fn(),
  });

  return {
    useSessionStore: () => stores.sessionStore,
  };
});

vi.mock('./stores/config', async () => {
  const { reactive } = await import('vue');

  const configStore = reactive({
    error: null as string | null,
    hasAgents: true,
    defaultAgentName: 'planner',
    loadConfig: loadConfigMock,
    setupHotReload: setupHotReloadMock,
    clearError() {
      configStore.error = null;
    },
  });

  return {
    useConfigStore: () => configStore,
    __configStore: configStore,
  };
});

vi.mock('./stores/appShell', async () => {
  const { reactive } = await import('vue');

  stores.shellStore = reactive({
    activeTab: 'home' as 'home' | 'agents' | 'knowledge' | 'schedules' | 'settings',
    setActiveTab(tab: 'home' | 'agents' | 'knowledge' | 'schedules' | 'settings') {
      setActiveTabMock(tab);
      stores.shellStore.activeTab = tab;
    },
  });

  return {
    useAppShellStore: () => stores.shellStore,
  };
});

vi.mock('./stores/embeddings', async () => {
  const { reactive } = await import('vue');

  const embeddingsStore = reactive({
    status: null as unknown,
    fetchStatus: vi.fn().mockResolvedValue(undefined),
  });

  return {
    useEmbeddingsStore: () => embeddingsStore,
  };
});

vi.mock('./lib/host', () => ({
  canPickFolder: () => false,
  pickFolder: vi.fn(),
  loadKvStore: loadKvStoreMock,
}));

vi.mock('./lib/telemetry', () => ({
  initTelemetry: initTelemetryMock,
}));

import App from './App.vue';

function getSessionStore() {
  return stores.sessionStore as any;
}

function getShellStore() {
  return stores.shellStore as any;
}

const AgentSelectorStub = defineComponent({
  emits: ['select', 'update:selected'],
  template:
    '<button class="select-agent" @click="$emit(\'update:selected\', \'planner\'); $emit(\'select\', \'planner\')">select agent</button>',
});

const HomeViewStub = defineComponent({
  emits: ['new-session', 'resume-session'],
  setup(_, { emit }) {
    return {
      emitNewSession() {
        emit('new-session');
      },
      emitResumeSession() {
        emit('resume-session', resumeTarget);
      },
    };
  },
  template: `
    <div class="home-view-stub">
      <button class="emit-new-session" @click="emitNewSession">new</button>
      <button class="emit-resume-session" @click="emitResumeSession">resume</button>
    </div>
  `,
});

const passthroughStub = defineComponent({
  template: '<div />',
});

const mountedWrappers: Array<ReturnType<typeof mount>> = [];

function matchMediaStub(): MediaQueryList {
  return {
    matches: false,
    media: '(max-width: 800px)',
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn().mockReturnValue(true),
  } as unknown as MediaQueryList;
}

function mountApp() {
  const wrapper = mount(App, {
    attachTo: document.body,
    global: {
      stubs: {
        AgentSelector: AgentSelectorStub,
        AuthMethodDialog: passthroughStub,
        ChatView: passthroughStub,
        HomeView: HomeViewStub,
        KnowledgeView: passthroughStub,
        OnboardingWizard: passthroughStub,
        PermissionDialog: passthroughStub,
        SchedulesView: passthroughStub,
        SessionList: passthroughStub,
        SettingsView: passthroughStub,
        StartupProgress: passthroughStub,
        TabBar: passthroughStub,
        TrafficMonitor: passthroughStub,
      },
    },
  });

  mountedWrappers.push(wrapper);
  return wrapper;
}

async function settle() {
  await flushPromises();
  await nextTick();
}

async function primeNewSessionInputs(wrapper: ReturnType<typeof mount>) {
  const vm = wrapper.vm as unknown as {
    selectedAgent: string;
    selectedCwd: string;
  };

  vm.selectedAgent = 'planner';
  vm.selectedCwd = 'C:\\worktree';
  await nextTick();
}

beforeEach(() => {
  createSessionMock.mockReset();
  resumeSessionMock.mockReset();
  setActiveTabMock.mockClear();
  loadConfigMock.mockReset();
  loadConfigMock.mockResolvedValue(undefined);
  setupHotReloadMock.mockReset();
  setupHotReloadMock.mockResolvedValue(undefined);
  initStoreMock.mockReset();
  initStoreMock.mockResolvedValue(undefined);
  initTelemetryMock.mockReset();
  initTelemetryMock.mockResolvedValue(undefined);

  loadKvStoreMock.mockReset();
  loadKvStoreMock.mockResolvedValue({
    get: vi.fn().mockResolvedValue(undefined),
    set: vi.fn().mockResolvedValue(undefined),
    save: vi.fn().mockResolvedValue(undefined),
  });

  const sessionStore = getSessionStore();
  const shellStore = getShellStore();

  sessionStore.isConnected = false;
  sessionStore.isLoading = false;
  sessionStore.isConnecting = false;
  sessionStore.isReconnecting = false;
  sessionStore.error = null;
  sessionStore.currentSession = null;

  shellStore.activeTab = 'home';

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation(matchMediaStub),
  });
});

afterEach(() => {
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }

  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('App home-tab navigation', () => {
  it('switches to agents when resuming from Home while already connected', async () => {
    const sessionStore = getSessionStore();
    const shellStore = getShellStore();

    sessionStore.isConnected = true;
    resumeSessionMock.mockResolvedValue(undefined);

    const wrapper = mountApp();
    await settle();

    expect(shellStore.activeTab).toBe('home');

    await wrapper.get('.emit-resume-session').trigger('click');
    await settle();

    expect(resumeSessionMock).toHaveBeenCalledWith(resumeTarget);
    expect(setActiveTabMock).toHaveBeenCalledWith('agents');
    expect(shellStore.activeTab).toBe('agents');
  });

  it('switches to agents when starting from Home while already connected', async () => {
    const sessionStore = getSessionStore();
    const shellStore = getShellStore();

    sessionStore.isConnected = true;
    createSessionMock.mockResolvedValue(undefined);

    const wrapper = mountApp();
    await settle();

    await primeNewSessionInputs(wrapper);
    setActiveTabMock.mockClear();
    shellStore.activeTab = 'home';

    await wrapper.get('.emit-new-session').trigger('click');
    await settle();

    expect(createSessionMock).toHaveBeenCalledWith('planner', 'C:\\worktree');
    expect(setActiveTabMock).toHaveBeenCalledWith('agents');
    expect(shellStore.activeTab).toBe('agents');
  });

  it('still lands on agents on a cold start session launch', async () => {
    const sessionStore = getSessionStore();
    const shellStore = getShellStore();

    sessionStore.isConnected = false;
    createSessionMock.mockImplementation(async () => {
      sessionStore.isConnected = true;
    });

    const wrapper = mountApp();
    await settle();

    await primeNewSessionInputs(wrapper);
    (wrapper.vm as unknown as { selectedCwd: string }).selectedCwd = 'C:\\cold-start';
    await nextTick();

    await wrapper.get('.emit-new-session').trigger('click');
    await settle();

    expect(createSessionMock).toHaveBeenCalledWith('planner', 'C:\\cold-start');
    expect(shellStore.activeTab).toBe('agents');
  });
});
