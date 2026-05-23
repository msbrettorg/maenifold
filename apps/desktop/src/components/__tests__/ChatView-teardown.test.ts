// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import ChatView from '../ChatView.vue';

const mocks = vi.hoisted(() => ({
  renderMermaidBlocks: vi.fn(async () => {}),
  sessionStore: null as unknown as {
    activeSession: Record<string, unknown>;
    currentSession: Record<string, unknown>;
    messageList: unknown[];
    isLoading: boolean;
    configOptions: unknown[];
    availableCommands: unknown[];
    currentPlan: null;
    pendingPermissionCount: number;
    agentPromptCapabilities: Record<string, unknown>;
    isConnected: boolean;
    setSessionMode: ReturnType<typeof vi.fn>;
    sendPrompt: ReturnType<typeof vi.fn>;
    cancelOperation: ReturnType<typeof vi.fn>;
    clearMessages: ReturnType<typeof vi.fn>;
  },
  agentStore: {
    selectedSession: null,
    start: vi.fn(async () => {}),
    stop: vi.fn(async () => {}),
    refreshSelectedSession: vi.fn(async () => {}),
  },
}));

vi.mock('../../stores/session', async () => {
  const { reactive } = await import('vue');
  const store = reactive({
    activeSession: {
      sessionId: 'session-1',
      agentName: 'test-agent',
      cwd: '/',
      isConnected: true,
      isLoading: false,
      messages: [],
      availableCommands: [],
      configOptions: [],
      currentPlan: null,
      pendingPermissions: [],
      agentPromptCapabilities: {},
      currentModeId: 'agent',
      modes: [],
    },
    currentSession: {
      id: 'saved-session',
      agentName: 'test-agent',
      sessionId: 'session-1',
      title: 'Teardown test',
      lastUpdated: 1,
      cwd: '/',
    },
    messageList: [],
    isLoading: false,
    configOptions: [],
    availableCommands: [],
    currentPlan: null,
    pendingPermissionCount: 0,
    agentPromptCapabilities: {},
    isConnected: true,
    setSessionMode: vi.fn(),
    sendPrompt: vi.fn(async () => true),
    cancelOperation: vi.fn(),
    clearMessages: vi.fn(),
  });
  mocks.sessionStore = store;
  return { useSessionStore: () => store };
});

vi.mock('../../stores/agent', () => ({
  useAgentStore: () => mocks.agentStore,
}));

vi.mock('../../stores/confirm-dialog', () => ({
  useConfirmDialogStore: () => ({}),
}));

vi.mock('../../stores/app-shell', () => ({
  useAppShellStore: () => ({}),
}));

vi.mock('../../composables/useMermaid', () => ({
  useMermaid: () => ({ renderMermaidBlocks: mocks.renderMermaidBlocks }),
}));

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => ({
    connected: { value: false },
    connect: vi.fn(async () => {}),
    disconnect: vi.fn(async () => {}),
    callTool: vi.fn(async () => ({})),
  }),
}));

vi.mock('../../composables/useChatDraft', async () => {
  const { ref } = await import('vue');
  return { useChatDraft: () => ({ inputText: ref(''), clearDraft: vi.fn() }) };
});

vi.mock('../../composables/useEscCancelWhileStreaming', () => ({
  useEscCancelWhileStreaming: vi.fn(),
}));

vi.mock('../../composables/useChatReconnect', () => ({
  useChatReconnect: () => ({ canReconnect: { value: false }, reconnect: vi.fn(async () => {}) }),
}));

vi.mock('../../composables/useMcpPromptCommands', async () => {
  const actual = await vi.importActual<typeof import('../../composables/useMcpPromptCommands')>('../../composables/useMcpPromptCommands');
  return { ...actual, useMcpPromptCommands: () => ({ commands: { value: [] } }) };
});

function makeMessage(id: string) {
  return { id, role: 'assistant' as const, content: 'hello', timestamp: Number(id) };
}

function mountChatView() {
  return mount(ChatView, {
    global: {
      stubs: {
        Button: { template: '<button><slot /></button>' },
        Card: { template: '<section><slot /></section>' },
        ScrollArea: { template: '<div><slot /></div>' },
        ChatInputBar: { template: '<div />' },
        ChatMessageList: { template: '<div />' },
        ConfigOptionPicker: true,
        McpPromptPicker: true,
        ModePicker: true,
        SessionConfigPanel: true,
        SessionModeSwitcher: true,
        Settings: true,
      },
    },
  });
}

afterEach(() => {
  mocks.renderMermaidBlocks.mockClear();
  mocks.agentStore.start.mockClear();
  mocks.agentStore.stop.mockClear();
  mocks.agentStore.refreshSelectedSession.mockClear();
  if (mocks.sessionStore) {
    mocks.sessionStore.messageList = [];
    mocks.sessionStore.isLoading = false;
  }
  vi.restoreAllMocks();
});

describe('ChatView teardown guards', () => {
  it('skips the async message rerender after unmount', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const wrapper = mountChatView();

    await nextTick();
    await flushPromises();
    mocks.renderMermaidBlocks.mockClear();

    mocks.sessionStore.messageList = [makeMessage('1')];
    await nextTick();
    wrapper.unmount();
    await nextTick();
    await flushPromises();

    expect(mocks.renderMermaidBlocks).not.toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
    expect(warnSpy).not.toHaveBeenCalled();
  });
});
