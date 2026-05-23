// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import ChatView from '../ChatView.vue';

const mocks = vi.hoisted(() => {
  const refValue = <T>(value: T) => ({ __v_isRef: true, value });
  const currentSession = {
    id: 'saved-session',
    agentName: 'test-agent',
    sessionId: 'remote-session',
    title: 'Mode test',
    lastUpdated: 1,
    cwd: '/',
  };
  const activeSession = {
    sessionId: 'local-session',
    agentName: 'test-agent',
    cwd: '/',
    session: currentSession,
    isConnected: true,
    isLoading: false,
    messages: [],
    availableCommands: [],
    configOptions: [],
    currentPlan: null,
    pendingPermissions: [],
    agentPromptCapabilities: {},
    currentModeId: 'agent',
    modes: [
      { id: 'agent', name: 'Agent', description: 'Default mode', _meta: null },
      { id: 'plan', name: 'Plan', description: 'Plan mode', _meta: null },
    ],
  };

  return {
    refValue,
    sessionStore: {
      activeSession,
      currentSession,
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
    },
    agentStore: {
      selectedSession: null,
      start: vi.fn(async () => {}),
      stop: vi.fn(async () => {}),
      refreshSelectedSession: vi.fn(async () => {}),
    },
  };
});

vi.mock('../../stores/session', () => ({
  useSessionStore: () => mocks.sessionStore,
}));

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
  useMermaid: () => ({ renderMermaidBlocks: vi.fn(async () => {}) }),
}));

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => ({
    connected: mocks.refValue(false),
    connect: vi.fn(async () => {}),
    disconnect: vi.fn(async () => {}),
    callTool: vi.fn(async () => ({})),
  }),
}));

vi.mock('../../composables/useChatDraft', () => ({
  useChatDraft: () => ({ inputText: mocks.refValue(''), clearDraft: vi.fn() }),
}));

vi.mock('../../composables/useEscCancelWhileStreaming', () => ({
  useEscCancelWhileStreaming: vi.fn(),
}));

vi.mock('../../composables/useChatReconnect', () => ({
  useChatReconnect: () => ({ canReconnect: mocks.refValue(false), reconnect: vi.fn(async () => {}) }),
}));

vi.mock('../../composables/useMcpPromptCommands', async () => {
  const actual = await vi.importActual<typeof import('../../composables/useMcpPromptCommands')>('../../composables/useMcpPromptCommands');
  return {
    ...actual,
    useMcpPromptCommands: () => ({ commands: mocks.refValue([]) }),
  };
});

function mountChatView() {
  return mount(ChatView, {
    global: {
      stubs: {
        Button: { template: '<button data-slot="button"><slot /></button>' },
        Card: { template: '<section><slot /></section>' },
        ScrollArea: { template: '<div><slot /></div>' },
        ChatInputBar: { template: '<div data-testid="chat-input-bar" />' },
        ChatMessageList: { template: '<div data-testid="chat-message-list" />' },
        ConfigOptionPicker: true,
        McpPromptPicker: true,
        SessionConfigPanel: true,
        SessionModeSwitcher: true,
        Settings: true,
      },
    },
  });
}

describe('ChatView ModePicker placement', () => {
  it('renders the ModePicker pill row below ChatInputBar when session modes are available', () => {
    const wrapper = mountChatView();
    const input = wrapper.get('[data-testid="chat-input-bar"]').element;
    const modePicker = wrapper.get('[data-testid="mode-picker"]').element;

    expect(wrapper.text()).toContain('Agent');
    expect(wrapper.text()).toContain('Plan');
    expect(input.compareDocumentPosition(modePicker) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});
