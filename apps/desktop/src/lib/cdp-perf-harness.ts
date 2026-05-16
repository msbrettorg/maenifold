import type { SessionNotification } from '@agentclientprotocol/sdk';
import { getActivePinia, storeToRefs } from 'pinia';
import { nextTick, reactive } from 'vue';
import { useMessageHandler } from '../composables/useMessageHandler';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import { usePipelineStore } from '../stores/pipeline';
import { useSessionStore } from '../stores/session';
import { useWorkflowStore } from '../stores/workflow';
import { createEmptySessionState, type ChatMessage, type SavedSession, type SessionState } from './types';

type H2Args = { toolCallCount: number; targetIndex: number };
type H3Args = { chunkSizes: number[]; chunkLength: number };
type H7Args = { notificationCount: number };
type BaselineArgs = { chunkCount: number; chunkLength: number; acpUiRoot?: string };
type Harness = {
  measureH2(args: H2Args): Promise<{ siblingUpdates: number; targetUpdates: number; toolCallCount: number }>;
  measureH3(args: H3Args): Promise<{ durations: Record<string, number>; ratio400to200: number; bottleneckConfirmed: boolean }>;
  measureH4(): Promise<{ totalRenderMs: number; settleFrames: number; blockCount: number; settled: boolean }>;
  measureH6(): Promise<{ changedSliceSubscribers: number; crossStoreMutations: number; firedSubscribers: string[] }>;
  measureH7(args: H7Args): Promise<{ notificationCount: number; frames: number; avgNotificationsPerFrame: number }>;
  measureAcpUiBaseline(args: BaselineArgs): Promise<{ oursMs: number; dispatcherBaselineMs: number; ratio: number }>;
};

declare global {
  interface Window {
    __MAENIFOLD_CDP_PERF__?: Harness;
  }
  var __MAENIFOLD_CDP_PERF__: Harness | undefined;
}

const raf = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const CHAT_LOG_SELECTOR = '[role="log"][aria-label="Chat messages"], [role="log"]';
const TOOL_CALL_SELECTOR = '[role="article"][aria-label$=" tool call"]';

async function flushUi(frames = 2): Promise<void> {
  await nextTick();
  for (let i = 0; i < frames; i += 1) await raf();
  await nextTick();
}

function requireChatLogRoot(): HTMLElement {
  const root = document.querySelector<HTMLElement>(CHAT_LOG_SELECTOR);
  if (!root) throw new Error('Chat log container not found');
  return root;
}

function findToolCallNode(target: Node | null): HTMLElement | null {
  const element = target instanceof Element ? target : target?.parentElement ?? null;
  return element?.closest<HTMLElement>(TOOL_CALL_SELECTOR) ?? null;
}

async function measureRenderSettle(
  root: HTMLElement,
  mutate: () => void | Promise<void>,
  maxFrames = 8,
): Promise<{ frames: number; settled: boolean }> {
  let mutations = 0;
  const observer = new MutationObserver(() => {
    mutations += 1;
  });
  observer.observe(root, { subtree: true, childList: true, attributes: true, characterData: true });
  try {
    await mutate();
    let frames = 0;
    let quietFrames = 0;
    let lastMutations = -1;
    while (frames < maxFrames && quietFrames < 2) {
      await flushUi(1);
      frames += 1;
      quietFrames = mutations === lastMutations ? quietFrames + 1 : 0;
      lastMutations = mutations;
    }
    return { frames, settled: quietFrames >= 2 };
  } finally {
    observer.disconnect();
  }
}

function makeAgentChunk(text: string): SessionNotification {
  return { sessionId: 'remote-harness', update: { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text } } } as SessionNotification;
}
function makeToolCall(toolCallId: string, title: string): SessionNotification {
  return { sessionId: 'remote-harness', update: { sessionUpdate: 'tool_call', toolCallId, title, kind: 'read', status: 'pending' } } as SessionNotification;
}
function makeToolCallUpdate(toolCallId: string, status: 'in_progress' | 'completed'): SessionNotification {
  return { sessionId: 'remote-harness', update: { sessionUpdate: 'tool_call_update', toolCallId, status } } as SessionNotification;
}
function makeSavedSession(id: string, messages: ChatMessage[]): SavedSession {
  return { id, agentName: 'Harness', sessionId: id, title: 'Harness', lastUpdated: Date.now(), cwd: '/tmp', messages };
}
function createHarnessSession(id: string): SessionState {
  const session = reactive(createEmptySessionState(id, 'Harness', '/tmp'));
  session.isConnected = true;
  session.session = makeSavedSession(id, session.messages);
  return session;
}

function runAcpUiDispatcherReference(chunks: string[]): number {
  const messages: Array<{ id: string; role: 'assistant'; content: string; timestamp: number; toolCalls: unknown[] }> = [];
  const start = performance.now();
  for (const text of chunks) {
    const last = messages[messages.length - 1];
    if (last) last.content += text;
    else messages.push({ id: crypto.randomUUID(), role: 'assistant', content: text, timestamp: Date.now(), toolCalls: [] });
  }
  return performance.now() - start;
}

export function installCdpPerfHarness(): void {
  if (globalThis.__MAENIFOLD_CDP_PERF__) return;
  const pinia = getActivePinia();
  if (!pinia) return;
  const sessionStore = useSessionStore(pinia);
  const appShellStore = useAppShellStore(pinia);
  const { sessions, activeSessionId, savedSessions } = storeToRefs(sessionStore);
  const handler = useMessageHandler({
    sessions,
    bridges: new Map(),
    activeSessionId,
    savedSessions,
    saveSessionsToStore: async () => {},
    cancelPermission: sessionStore.cancelPermission,
  });
  async function withTempSession<T>(run: (id: string, session: SessionState) => Promise<T>): Promise<T> {
    const previousSessions = sessions.value;
    const previousActive = activeSessionId.value;
    const previousTab = appShellStore.activeTab;
    const id = `cdp-perf-${crypto.randomUUID()}`;
    const session = createHarnessSession(id);
    try {
      const nextSessions = new Map(previousSessions);
      nextSessions.set(id, session);
      sessions.value = nextSessions;
      activeSessionId.value = id;
      appShellStore.setActiveTab('agents');
      await flushUi();
      return await run(id, session);
    } finally {
      sessions.value = previousSessions;
      activeSessionId.value = previousActive;
      appShellStore.setActiveTab(previousTab);
      await flushUi();
    }
  }
  const harness: Harness = {
    async measureH2({ toolCallCount, targetIndex }) {
      return await withTempSession(async (id, session) => {
        session.isLoading = true;
        handler.handleSessionUpdate(id, makeAgentChunk('running tools'));
        for (let i = 0; i < toolCallCount; i += 1) handler.handleSessionUpdate(id, makeToolCall(`tool-${i}`, `Tool ${i}`));
        await flushUi();
        const items = Array.from(document.querySelectorAll<HTMLElement>(TOOL_CALL_SELECTOR));
        const target = items[targetIndex] ?? null;
        const logRoot = requireChatLogRoot();
        const siblings = new Set<HTMLElement>();
        let targetUpdates = 0;
        const observer = new MutationObserver((records) => {
          for (const record of records) {
            const node = findToolCallNode(record.target);
            if (!node) continue;
            if (node === target) targetUpdates += 1;
            else siblings.add(node);
          }
        });
        observer.observe(logRoot, { subtree: true, childList: true, attributes: true, characterData: true });
        handler.handleSessionUpdate(id, makeToolCallUpdate(`tool-${targetIndex}`, 'completed'));
        await flushUi();
        observer.disconnect();
        return { siblingUpdates: siblings.size, targetUpdates: targetUpdates > 0 ? 1 : 0, toolCallCount };
      });
    },
    async measureH3({ chunkSizes, chunkLength }) {
      const durations: Record<string, number> = {};
      for (const size of chunkSizes) {
        durations[String(size)] = await withTempSession(async (id, session) => {
          session.isLoading = true;
          const start = performance.now();
          for (let i = 0; i < size; i += 1) handler.handleSessionUpdate(id, makeAgentChunk(`${'x'.repeat(chunkLength)}${i}`));
          await flushUi();
          return performance.now() - start;
        });
      }
      const ratio400to200 = durations['200'] > 0 ? durations['400'] / durations['200'] : 0;
      return { durations, ratio400to200, bottleneckConfirmed: ratio400to200 > 3 };
    },
    async measureH4() {
      return await withTempSession(async (_id, session) => {
        const code = Array.from({ length: 200 }, (_, i) => `const line${i} = ${i};`).join('\n');
        const list = Array.from({ length: 10 }, (_, i) => `- item ${i + 1}`).join('\n');
        const content = `Intro with **bold**, *italic*, and [link](https://example.com).\n\n${list}\n\n\`\`\`ts\n${code}\n\`\`\``;
        const logRoot = requireChatLogRoot();
        const start = performance.now();
        const { frames: settleFrames, settled } = await measureRenderSettle(logRoot, () => {
          session.messages.push({ id: crypto.randomUUID(), role: 'assistant', content, timestamp: Date.now(), toolCalls: [] });
        });
        const markdown = Array.from(document.querySelectorAll<HTMLElement>('article.prose')).at(-1);
        return { totalRenderMs: performance.now() - start, settleFrames, blockCount: markdown?.children.length ?? 0, settled };
      });
    },
    async measureH6() {
      return await withTempSession(async (id, session) => {
        const stores = [
          sessionStore,
          appShellStore,
          useKnowledgeStore(pinia),
          usePipelineStore(pinia),
          useWorkflowStore(pinia),
        ];
        const fired = new Set<string>();
        // Pinia $subscribe reports store mutations, not downstream watcher/effect executions.
        const stops = stores.map((store) => store.$subscribe(() => fired.add(store.$id), { detached: true }));
        try {
          session.isLoading = true;
          handler.handleSessionUpdate(id, makeAgentChunk('slice probe'));
          await flushUi();
        } finally {
          stops.forEach((stop) => stop());
        }
        const firedSubscribers = [...fired];
        return {
          changedSliceSubscribers: firedSubscribers.filter((name) => name === 'session').length,
          crossStoreMutations: firedSubscribers.filter((name) => name !== 'session').length,
          firedSubscribers,
        };
      });
    },
    async measureH7({ notificationCount }) {
      return await withTempSession(async (id, session) => {
        session.isLoading = true;
        const logRoot = requireChatLogRoot();
        const { frames } = await measureRenderSettle(logRoot, () => {
          for (let i = 0; i < notificationCount; i += 1) handler.handleSessionUpdate(id, makeAgentChunk(`burst-${i}`));
        });
        const settledFrames = Math.max(frames, 1);
        return { notificationCount, frames: settledFrames, avgNotificationsPerFrame: notificationCount / settledFrames };
      });
    },
    async measureAcpUiBaseline({ chunkCount, chunkLength, acpUiRoot }) {
      void acpUiRoot;
      const chunks = Array.from({ length: chunkCount }, (_, i) => `${'x'.repeat(chunkLength)}${i}`);
      const oursMs = await withTempSession(async (id, session) => {
        session.isLoading = true;
        const start = performance.now();
        for (const chunk of chunks) handler.handleSessionUpdate(id, makeAgentChunk(chunk));
        await flushUi();
        return performance.now() - start;
      });
      // This is the same dispatcher-only baseline used in T-244.73, not a full acp-ui render benchmark.
      const dispatcherBaselineMs = runAcpUiDispatcherReference(chunks);
      return { oursMs, dispatcherBaselineMs, ratio: dispatcherBaselineMs > 0 ? oursMs / dispatcherBaselineMs : 0 };
    },
  };
  globalThis.__MAENIFOLD_CDP_PERF__ = harness;
  window.__MAENIFOLD_CDP_PERF__ = harness;
}
