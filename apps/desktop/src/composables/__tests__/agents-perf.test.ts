// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it } from 'vitest';
import { defineComponent, h, nextTick, onRenderTriggered, reactive, ref } from 'vue';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import ChatMessageList from '../../components/ChatMessageList.vue';
import { createEmptySessionState, type SavedSession, type SessionState } from '../../lib/types';
import { getMarkdownIt } from '../useMarkdownIt';
import { useMessageHandler } from '../useMessageHandler';

const TOTAL_STREAM_BUDGET_MS = 2_000;
const MAX_CHUNK_BUDGET_MS = 16;
const TOOL_STREAM_BUDGET_MS = 750;
const BIG_MESSAGE_BUDGET_MS = 1_000;

function makeHandler(session: SessionState) {
  const sessions = ref(new Map([[session.sessionId, session]]));
  return useMessageHandler({
    sessions,
    bridges: new Map(),
    activeSessionId: ref(session.sessionId),
    savedSessions: ref<SavedSession[]>([]),
    saveSessionsToStore: async () => {},
    cancelPermission: () => {},
  });
}

function makeAgentChunk(text: string): SessionNotification {
  return {
    sessionId: 'remote-1',
    update: {
      sessionUpdate: 'agent_message_chunk',
      content: { type: 'text', text },
    },
  } as SessionNotification;
}

function makeToolCall(toolCallId: string, title = toolCallId): SessionNotification {
  return {
    sessionId: 'remote-1',
    update: {
      sessionUpdate: 'tool_call',
      toolCallId,
      title,
      kind: 'read',
      status: 'pending',
    },
  } as SessionNotification;
}

function makeToolCallUpdate(
  toolCallId: string,
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled',
): SessionNotification {
  return {
    sessionId: 'remote-1',
    update: {
      sessionUpdate: 'tool_call_update',
      toolCallId,
      status,
    },
  } as SessionNotification;
}

async function waitForVueWork(): Promise<void> {
  await nextTick();
  await new Promise((resolve) => window.setTimeout(resolve, 0));
  await nextTick();
}

async function mountMessageList(session: SessionState) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const renderTriggers = ref(0);
  const Host = defineComponent({
    name: 'AgentsPerfHost',
    setup() {
      onRenderTriggered(() => {
        renderTriggers.value += 1;
      });
      return () => h(ChatMessageList, {
        messages: session.messages,
        currentPlan: session.currentPlan,
        expandedThoughtIds: [],
        expandedToolCallIds: [],
        pendingPermissionCount: session.pendingPermissions.length,
        isLoading: session.isLoading,
      });
    },
  });

  const wrapper = mount(Host, {
    attachTo: document.body,
    global: {
      plugins: [pinia],
    },
  });
  await waitForVueWork();
  return { wrapper, renderTriggers };
}

function runAcpUiDispatcherReference(chunks: string[]): number {
  const messages: Array<{ id: string; role: 'assistant'; content: string; timestamp: number; toolCalls: unknown[] }> = [];
  const start = performance.now();
  for (const text of chunks) {
    const lastMsg = messages[messages.length - 1];
    if (lastMsg && lastMsg.role === 'assistant') {
      lastMsg.content += text;
    } else {
      messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: text,
        timestamp: Date.now(),
        toolCalls: [],
      });
    }
  }
  return performance.now() - start;
}

describe('Agents tab rendering performance', () => {
  it('handles 100 agent_message_chunk notifications under the frame budget', async () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    session.isLoading = true;
    const handler = makeHandler(session);
    const { wrapper, renderTriggers } = await mountMessageList(session);
    const chunks = Array.from({ length: 100 }, (_, index) => `chunk-${index.toString().padStart(3, '0')} text `);
    const acpUiReferenceMs = runAcpUiDispatcherReference(chunks);
    const chunkTimes: number[] = [];
    const start = performance.now();

    for (const chunk of chunks) {
      const before = performance.now();
      handler.handleSessionUpdate(session.sessionId, makeAgentChunk(chunk));
      await waitForVueWork();
      chunkTimes.push(performance.now() - before);
    }

    const totalMs = performance.now() - start;
    const maxChunkMs = Math.max(...chunkTimes);
    const averageChunkMs = totalMs / chunks.length;

    console.info(
      `[agents-perf] chunks=100 total=${totalMs.toFixed(2)}ms max=${maxChunkMs.toFixed(2)}ms avg=${averageChunkMs.toFixed(2)}ms renderTriggers=${renderTriggers.value} acpUiReference=${acpUiReferenceMs.toFixed(2)}ms`,
    );

    expect(session.messages).toHaveLength(1);
    expect(session.messages[0]?.content).toBe(chunks.join(''));
    expect(totalMs).toBeLessThan(TOTAL_STREAM_BUDGET_MS);
    expect(maxChunkMs).toBeLessThan(MAX_CHUNK_BUDGET_MS);
    wrapper.unmount();
  });

  it('handles a tool_call_update stream without replacing sibling tool-call objects', async () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    session.isLoading = true;
    const handler = makeHandler(session);
    handler.handleSessionUpdate(session.sessionId, makeAgentChunk('running tools'));
    for (let index = 0; index < 10; index += 1) {
      handler.handleSessionUpdate(session.sessionId, makeToolCall(`tool-${index}`, `Tool ${index}`));
    }

    const siblingReferences = new Map(
      session.messages[0]?.toolCalls?.map((toolCall) => [toolCall.toolCallId, toolCall]) ?? [],
    );
    const { wrapper, renderTriggers } = await mountMessageList(session);
    const statuses = ['in_progress', 'completed'] as const;
    const start = performance.now();

    for (let index = 0; index < 50; index += 1) {
      handler.handleSessionUpdate(
        session.sessionId,
        makeToolCallUpdate('tool-4', statuses[index % statuses.length]),
      );
      await waitForVueWork();
    }

    const totalMs = performance.now() - start;
    const toolCalls = session.messages[0]?.toolCalls ?? [];
    const changed = toolCalls.find((toolCall) => toolCall.toolCallId === 'tool-4');

    console.info(
      `[agents-perf] toolUpdates=50 total=${totalMs.toFixed(2)}ms renderTriggers=${renderTriggers.value}`,
    );

    expect(changed?.status).toBe('completed');
    for (const toolCall of toolCalls) {
      if (toolCall.toolCallId === 'tool-4') continue;
      expect(toolCall).toBe(siblingReferences.get(toolCall.toolCallId));
    }
    expect(totalMs).toBeLessThan(TOOL_STREAM_BUDGET_MS);
    wrapper.unmount();
  });

  it('renders a 2000-token agent message with an embedded code block under budget', async () => {
    await getMarkdownIt();
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    session.isLoading = false;
    const handler = makeHandler(session);
    const prose = Array.from({ length: 2_000 }, (_, index) => `token${index}`).join(' ');
    const code = [
      '```ts',
      'export function answer(): number {',
      '  return 42;',
      '}',
      '```',
    ].join('\n');
    handler.handleSessionUpdate(session.sessionId, makeAgentChunk(`${prose}\n\n${code}`));
    const start = performance.now();
    const { wrapper } = await mountMessageList(session);
    await waitForVueWork();
    const totalMs = performance.now() - start;

    console.info(`[agents-perf] bigMessageTokens=2000 total=${totalMs.toFixed(2)}ms`);

    expect(wrapper.text()).toContain('token1999');
    expect(totalMs).toBeLessThan(BIG_MESSAGE_BUDGET_MS);
    wrapper.unmount();
  });
});
