// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import ChatView from '../ChatView.vue';
import type { AvailableCommand } from '../../lib/types';
import type { SlashCommandNotice } from '../../lib/slash-command-notice';

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
    mcpPromptCommands: refValue([] as AvailableCommand[]),
    sessionStore: {
      activeSession,
      currentSession,
      messageList: [],
      isLoading: false,
      configOptions: [],
      availableCommands: [] as AvailableCommand[],
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
    useMcpPromptCommands: () => ({ commands: mocks.mcpPromptCommands }),
  };
});

const chatInputBarProps = vi.hoisted(() => ({ value: null as null | Record<string, unknown> }));

function mountChatView() {
  return mount(ChatView, {
    global: {
      stubs: {
        Button: { template: '<button data-slot="button"><slot /></button>' },
        Card: { template: '<section><slot /></section>' },
        ScrollArea: { template: '<div><slot /></div>' },
        ChatInputBar: { props: ['availableCommands', 'commandPaletteNotice'], template: '<div data-testid="chat-input-bar" />', setup: (props: Record<string, unknown>) => { chatInputBarProps.value = props; return {}; } },
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

function currentNotice(): SlashCommandNotice | null {
  return (chatInputBarProps.value?.commandPaletteNotice as SlashCommandNotice | null | undefined) ?? null;
}

describe('ChatView', () => {
  beforeEach(() => {
    chatInputBarProps.value = null;
    mocks.mcpPromptCommands.value = [];
    mocks.sessionStore.availableCommands = [];
    mocks.sessionStore.activeSession.agentName = 'test-agent';
  });

  it('renders the ModePicker pill row below ChatInputBar when session modes are available', () => {
    const wrapper = mountChatView();
    const input = wrapper.get('[data-testid="chat-input-bar"]').element;
    const modePicker = wrapper.get('[data-testid="mode-picker"]').element;

    expect(wrapper.text()).toContain('Agent');
    expect(wrapper.text()).toContain('Plan');
    expect(input.compareDocumentPosition(modePicker) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('passes a command palette notice for copilot sessions with zero ACP commands', () => {
    mocks.sessionStore.activeSession.agentName = 'Copilot';
    mocks.sessionStore.availableCommands = [];
    mountChatView();
    expect(currentNotice()).toMatchObject({ heading: 'Slash commands unavailable', agentName: 'Copilot' });
  });

  it('clears the notice once ACP commands are available', () => {
    mocks.sessionStore.activeSession.agentName = 'Copilot';
    mocks.sessionStore.availableCommands = [{ name: 'model', description: 'Change model' }];
    mountChatView();
    expect(currentNotice()).toBeNull();
  });

  it('still merges MCP prompt commands into the available command list', () => {
    mocks.sessionStore.availableCommands = [{ name: 'model', description: 'Change model' }];
    mocks.mcpPromptCommands.value = [{ name: 'mcp.prompt', description: 'Prompt' }];
    mountChatView();
    expect((chatInputBarProps.value?.availableCommands as Array<{ name: string }>).map((command) => command.name)).toEqual(['model', 'mcp.prompt']);
  });
});
