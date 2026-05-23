// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, nextTick } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import ChatView from '../ChatView.vue';

const initializeMock = vi.hoisted(() => vi.fn());
const renderMock = vi.hoisted(() => vi.fn());
const mocks = vi.hoisted(() => ({
  sessionStore: null as unknown as {
    activeSession: Record<string, unknown>;
    currentSession: Record<string, unknown>;
    messageList: Array<{ id: string; content: string }>;
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

vi.mock('mermaid', () => ({
  default: {
    initialize: initializeMock,
    render: renderMock,
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

const ChatMessageListStub = defineComponent({
  props: {
    messages: {
      type: Array,
      default: () => [],
    },
  },
  methods: {
    mermaidSource(message: { content?: string }): string {
      return message.content?.match(/```mermaid\s+([\s\S]*?)```/m)?.[1]?.trim() ?? '';
    },
  },
  template: '<div><pre v-for="message in messages" :key="message.id"><code class="language-mermaid">{{ mermaidSource(message) }}</code></pre></div>',
});

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((fulfill) => { resolve = fulfill; });
  return { promise, resolve };
}

function primeMermaidRender() {
  const started = createDeferred<void>();
  const rendered = createDeferred<{ svg: string }>();
  renderMock.mockImplementationOnce(async () => {
    started.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));
    return rendered.promise;
  });
  return { started: started.promise, resolve: rendered.resolve };
}

function makeMessage(id: string) {
  return {
    id,
    role: 'assistant' as const,
    content: ['```mermaid', 'flowchart TD', '  A[Ready] --> B[Done]', '```'].join('\n'),
    timestamp: Number(id),
  };
}

function mountChatView() {
  return mount(ChatView, {
    global: {
      stubs: {
        Button: { template: '<button><slot /></button>' },
        Card: { template: '<section><slot /></section>' },
        ScrollArea: { template: '<div><slot /></div>' },
        ChatInputBar: { template: '<div />' },
        ChatMessageList: ChatMessageListStub,
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
  renderMock.mockReset();
  initializeMock.mockReset();
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
  it('does not mutate detached mermaid blocks when async rendering resumes after unmount', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const renderControl = primeMermaidRender();
    const wrapper = mountChatView();

    await nextTick();
    mocks.sessionStore.messageList = [makeMessage('1')];
    await nextTick();
    await renderControl.started;

    const detachedPre = wrapper.find('pre').element as HTMLPreElement;
    const replaceWithSpy = vi.spyOn(detachedPre, 'replaceWith');

    wrapper.unmount();
    await nextTick();
    expect(detachedPre.isConnected).toBe(false);

    renderControl.resolve({ svg: '<svg xmlns="http://www.w3.org/2000/svg"><text>Rendered</text></svg>' });
    await flushPromises();

    expect(replaceWithSpy).not.toHaveBeenCalled();
    expect(document.body.querySelector('.mermaid-rendered')).toBeNull();
    expect(errorSpy).not.toHaveBeenCalled();
  });
});
