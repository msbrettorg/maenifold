// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import SessionInspectorPanel from '../sessions/SessionInspectorPanel.vue';

const mocks = vi.hoisted(() => ({
  renderMermaidBlocks: vi.fn(async () => {}),
  sessionStore: null as unknown as {
    sessions: Map<string, unknown>;
    setActiveSession: ReturnType<typeof vi.fn>;
    cancelOperation: ReturnType<typeof vi.fn>;
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

vi.mock('@/composables/useMermaid', () => ({
  useMermaid: () => ({ renderMermaidBlocks: mocks.renderMermaidBlocks }),
}));

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
  return { id, role: 'assistant' as const, content: 'hello', timestamp: Number(id) };
}

function mountPanel() {
  return mount(SessionInspectorPanel, {
    props: { sessionId: 'session-1' },
    global: {
      stubs: {
        ChatMessageList: { template: '<div />' },
        ScrollArea: { template: '<div><slot /></div>' },
      },
    },
  });
}

afterEach(() => {
  mocks.renderMermaidBlocks.mockClear();
  if (mocks.sessionStore) {
    mocks.sessionStore.sessions = new Map();
  }
  vi.restoreAllMocks();
});

describe('SessionInspectorPanel teardown guards', () => {
  it('skips the async inspector rerender after unmount', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    mocks.sessionStore.sessions = new Map([['session-1', makeSession()]]);
    const wrapper = mountPanel();

    await nextTick();
    await flushPromises();
    mocks.renderMermaidBlocks.mockClear();

    mocks.sessionStore.sessions = new Map([['session-1', makeSession([makeMessage('1')])]]);
    await nextTick();
    wrapper.unmount();
    await nextTick();
    await flushPromises();

    expect(mocks.renderMermaidBlocks).not.toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
    expect(warnSpy).not.toHaveBeenCalled();
  });
});
