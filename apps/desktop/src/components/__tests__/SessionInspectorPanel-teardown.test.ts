// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, nextTick } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import SessionInspectorPanel from '../sessions/SessionInspectorPanel.vue';

const initializeMock = vi.hoisted(() => vi.fn());
const renderMock = vi.hoisted(() => vi.fn());
const mocks = vi.hoisted(() => ({
  sessionStore: null as unknown as {
    sessions: Map<string, unknown>;
    setActiveSession: ReturnType<typeof vi.fn>;
    cancelOperation: ReturnType<typeof vi.fn>;
  },
}));

vi.mock('mermaid', () => ({
  default: {
    initialize: initializeMock,
    render: renderMock,
  },
}));

vi.mock('pinia', async () => {
  const actual = await vi.importActual<typeof import('pinia')>('pinia');
  const { toRefs } = await import('vue');
  return { ...actual, storeToRefs: toRefs };
});

vi.mock('@/stores/session', async () => {
  const { reactive } = await import('vue');
  const store = reactive({
    sessions: new Map(),
    setActiveSession: vi.fn(),
    cancelOperation: vi.fn(async () => {}),
  });
  mocks.sessionStore = store;
  return { useSessionStore: () => store };
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

function makeSession(messages: unknown[] = []) {
  return {
    sessionId: 'session-1',
    messages,
    currentPlan: null,
    pendingPermissions: [],
    isLoading: false,
  };
}

function makeMessage(id: string) {
  return {
    id,
    role: 'assistant' as const,
    content: ['```mermaid', 'flowchart TD', '  A[Inspect] --> B[Done]', '```'].join('\n'),
    timestamp: Number(id),
  };
}

function mountPanel() {
  return mount(SessionInspectorPanel, {
    props: { sessionId: 'session-1' },
    global: {
      stubs: {
        ChatMessageList: ChatMessageListStub,
        ScrollArea: { template: '<div><slot /></div>' },
      },
    },
  });
}

afterEach(() => {
  renderMock.mockReset();
  initializeMock.mockReset();
  if (mocks.sessionStore) {
    mocks.sessionStore.sessions = new Map();
  }
  vi.restoreAllMocks();
});

describe('SessionInspectorPanel teardown guards', () => {
  it('does not mutate detached mermaid blocks when inspector teardown races async rendering', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const renderControl = primeMermaidRender();
    mocks.sessionStore.sessions = new Map([['session-1', makeSession()]]);
    const wrapper = mountPanel();

    await nextTick();
    mocks.sessionStore.sessions = new Map([['session-1', makeSession([makeMessage('1')])]]);
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
