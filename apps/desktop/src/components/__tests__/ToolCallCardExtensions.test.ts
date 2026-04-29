// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick, reactive, toRef } from 'vue';
import type { SessionNotification } from '@agentclientprotocol/sdk';

import ToolCallCard from '../ToolCallCard.vue';
import ToolCallCardExtensions from '../ToolCallCardExtensions.vue';
import { createEmptySessionState, type ToolCallInfo } from '../../lib/types';
import { useMessageHandler } from '../../composables/useMessageHandler';
import { useSessionStore } from '../../stores/session';

const base64 = 'aGVsbG8=';

vi.mock('vue-sonner', () => ({ toast: { error: vi.fn() } }));

function createToolCall(overrides: Partial<ToolCallInfo> = {}): ToolCallInfo {
  return {
    toolCallId: 'tool-1',
    title: 'Test tool',
    kind: 'read',
    status: 'in_progress',
    content: [],
    ...overrides,
  };
}

function mountExtensions(toolCall: ToolCallInfo) {
  return mount(ToolCallCardExtensions, {
    props: { toolCall, expanded: true },
    global: {
      plugins: [createPinia()],
      stubs: {
        KnowledgeNoteRenderer: { props: ['content'], template: '<article data-testid="knowledge-note">{{ content }}</article>' },
        MarkdownView: { props: ['content'], template: '<article data-testid="markdown-view">{{ content }}</article>' },
      },
    },
  });
}

function createStoreHandlerHarness() {
  setActivePinia(createPinia());
  const sessionStore = useSessionStore();
  const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
  sessionStore.sessions = new Map([[session.sessionId, session]]);
  sessionStore.activeSessionId = session.sessionId;
  const handler = useMessageHandler({
    sessions: toRef(sessionStore, 'sessions'),
    bridges: new Map(),
    activeSessionId: toRef(sessionStore, 'activeSessionId'),
    savedSessions: toRef(sessionStore, 'savedSessions'),
    saveSessionsToStore: async () => {},
    cancelPermission: () => {},
  });
  return { handler, session };
}

function sendToolCall(
  handler: ReturnType<typeof useMessageHandler>,
  update: Record<string, unknown>,
) {
  handler.handleSessionUpdate('local-1', {
    sessionId: 'remote-1',
    update: {
      sessionUpdate: 'tool_call',
      toolCallId: 'tool-1',
      title: 'Test tool',
      kind: 'read',
      status: 'pending',
      ...update,
    },
  } as unknown as SessionNotification);
}

function sendToolCallUpdate(
  handler: ReturnType<typeof useMessageHandler>,
  update: Record<string, unknown>,
) {
  handler.handleSessionUpdate('local-1', {
    sessionId: 'remote-1',
    update: {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'tool-1',
      ...update,
    },
  } as unknown as SessionNotification);
}

describe('ToolCallCardExtensions content blocks', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders text content type', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'content', content: { type: 'text', text: 'hello' } }],
    }));

    expect(wrapper.find('[aria-label="Text content"]').text()).toContain('hello');
  });

  it('renders image content type', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'content', content: { type: 'image', mimeType: 'image/png', data: base64 } }],
    }));

    const image = wrapper.find('img');
    expect(image.exists()).toBe(true);
    expect(image.attributes('src')).toBe(`data:image/png;base64,${base64}`);
  });

  it('renders audio content type', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'content', content: { type: 'audio', mimeType: 'audio/mp3', data: base64 } }],
    }));

    const audio = wrapper.find('audio');
    expect(audio.exists()).toBe(true);
    expect(audio.attributes('src')).toBe(`data:audio/mp3;base64,${base64}`);
  });

  it('renders resource_link content type', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{
        type: 'content',
        content: { type: 'resource_link', uri: 'file:///foo', name: 'foo.txt', mimeType: 'text/plain' },
      }],
    }));

    const link = wrapper.find('[data-testid="resource-link-chip"]');
    expect(link.exists()).toBe(true);
    expect(link.text()).toContain('foo.txt');
    expect(link.text()).toContain('file:///foo');
  });

  it('renders resource content type', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{
        type: 'content',
        content: {
          type: 'resource',
          resource: { uri: 'inline://x', mimeType: 'text/plain', text: 'inline body' },
        },
      }],
    }));

    const resource = wrapper.find('[aria-label="Resource content"]');
    expect(resource.exists()).toBe(true);
    expect(resource.text()).toContain('inline body');
  });
});

describe('ToolCallCardExtensions tool-call content blocks', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders diff content block with unified diff', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'diff', oldText: 'a\n', newText: 'b\n', path: '/x' }],
    }));

    const diff = wrapper.find('[data-testid="tool-call-diff"]');
    expect(diff.exists()).toBe(true);
    expect(diff.text()).toContain('--- /x');
    expect(diff.text()).toContain('+++ /x');
    expect(diff.text()).toContain('-a');
    expect(diff.text()).toContain('+b');
  });

  it('renders terminal content block with output text', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'terminal', output: 'hello world', exitCode: 0 }],
    }));

    expect(wrapper.find('[data-testid="terminal-output"]').text()).toContain('hello world');
  });

  it('renders terminal content block with exit code badge', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'terminal', output: 'oops', exitCode: 1 }],
    }));

    expect(wrapper.find('[data-testid="terminal-exit-code"]').text().toLowerCase()).toContain('exit 1');
  });

  it('renders terminal content block with truncated indicator', () => {
    const wrapper = mountExtensions(createToolCall({
      content: [{ type: 'terminal', output: 'x', truncated: true }],
    }));

    expect(wrapper.find('[data-testid="terminal-truncated"]').text().toLowerCase()).toContain('truncated');
  });

  it('falls back to JSON for unknown block types', () => {
    const block = { type: 'unknown_future_type', payload: { foo: 1 } };
    const wrapper = mountExtensions(createToolCall({ content: [block] }));

    expect(wrapper.find('[data-testid="unknown-tool-call-block"]').text()).toBe(JSON.stringify(block, null, 2));
  });
});

describe('Tool call lifecycle rendering', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders pending status correctly', () => {
    const wrapper = mount(ToolCallCard, { props: { toolCall: createToolCall({ status: 'pending' }) } });

    expect(wrapper.find('[data-testid="tool-call-status"]').text().toLowerCase()).toContain('pending');
  });

  it('renders in_progress status correctly', () => {
    const wrapper = mount(ToolCallCard, { props: { toolCall: createToolCall({ status: 'in_progress' }) } });

    expect(wrapper.find('[data-testid="tool-call-status"]').text().toLowerCase()).toContain('in progress');
  });

  it('renders completed status correctly', () => {
    const wrapper = mount(ToolCallCard, { props: { toolCall: createToolCall({ status: 'completed' }) } });

    expect(wrapper.find('[data-testid="tool-call-status"]').text().toLowerCase()).toContain('completed');
  });

  it('renders failed status correctly', () => {
    const wrapper = mount(ToolCallCard, { props: { toolCall: createToolCall({ status: 'failed' }) } });

    expect(wrapper.find('[data-testid="tool-call-status"]').text().toLowerCase()).toContain('failed');
  });

  it('updates tool call in place via tool_call_update', async () => {
    const { handler, session } = createStoreHandlerHarness();
    sendToolCall(handler, { status: 'pending' });
    const original = session.messages[0]?.toolCalls?.[0];

    sendToolCallUpdate(handler, {
      status: 'completed',
      content: [{ type: 'content', content: { type: 'image', mimeType: 'image/png', data: base64 } }],
    });
    await nextTick();

    expect(session.messages[0]?.toolCalls?.[0]).toBe(original);
    expect(original?.status).toBe('completed');
    const wrapper = mountExtensions(original as ToolCallInfo);
    expect(wrapper.find('img').attributes('src')).toBe(`data:image/png;base64,${base64}`);
  });

  it('does not append duplicate tool call on update', () => {
    const { handler, session } = createStoreHandlerHarness();

    sendToolCall(handler, { status: 'pending' });
    sendToolCall(handler, { status: 'in_progress', title: 'Updated title' });

    expect(session.messages[0]?.toolCalls).toHaveLength(1);
    expect(session.messages[0]?.toolCalls?.[0]).toMatchObject({
      toolCallId: 'tool-1',
      title: 'Updated title',
      status: 'in_progress',
    });
  });
});
