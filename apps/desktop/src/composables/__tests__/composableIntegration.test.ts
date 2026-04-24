// @vitest-environment jsdom
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick, reactive, toRef } from 'vue';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { createEmptySessionState } from '../../lib/types';
import { useSessionStore } from '../../stores/session';
import { useMessageHandler } from '../useMessageHandler';
import { useMemoryApi } from '../useMemoryApi';
import { useMcpClient } from '../useMcpClient';
import { useWorkflowTree } from '../useWorkflowTree';

interface ProbeResult {
  available: boolean;
  reason?: string;
}

interface MutableTask {
  mode?: 'run' | 'skip' | 'only' | 'todo';
}

interface MutableSuite extends MutableTask {
  tasks?: MutableTask[];
}

const SERVER_ORIGIN = 'http://127.0.0.1:8080';
const PROBE_TIMEOUT_MS = 3_000;

let savedLocation: Location;
let workflowUri: string | null = null;

function setWindowLocation(url: string): void {
  Object.defineProperty(window, 'location', {
    value: new URL(url),
    writable: true,
    configurable: true,
  });
}

function skipSuite(suite: MutableSuite): void {
  suite.mode = 'skip';
  suite.tasks?.forEach((task) => {
    task.mode = 'skip';
  });
}

async function probeMemoryApi(): Promise<ProbeResult> {
  const client = useMcpClient(SERVER_ORIGIN);
  const timeout = globalThis.setTimeout(() => {
    void client.disconnect();
  }, PROBE_TIMEOUT_MS);

  try {
    await client.connect();
    await client.callTool('RecentActivity', { filter: 'thinking', limit: 1 });
    return { available: true };
  } catch (error: unknown) {
    return {
      available: false,
      reason: error instanceof Error ? error.message : String(error),
    };
  } finally {
    globalThis.clearTimeout(timeout);
    await client.disconnect();
  }
}

async function findWorkflowUri(): Promise<string | null> {
  const api = useMemoryApi();
  const recent = await api.recentActivity({ filter: 'thinking', limit: 200 });
  const recentWorkflow = recent.items.find((item) => item.uri.includes('/thinking/workflow/'));
  if (recentWorkflow) {
    return recentWorkflow.uri;
  }

  const search = await api.searchMemories('workflow', {
    folder: 'thinking/workflow',
    pageSize: 25,
  });
  return search.results[0]?.uri ?? null;
}

function createHandlerHarness() {
  setActivePinia(createPinia());
  const store = useSessionStore();
  const sessions = toRef(store, 'sessions');
  const activeSessionId = toRef(store, 'activeSessionId');
  const savedSessions = toRef(store, 'savedSessions');

  const localId = 'test-session-1';
  const session = reactive(createEmptySessionState(localId, 'test-agent', '/test'));
  sessions.value = new Map([[localId, session]]);
  activeSessionId.value = localId;
  savedSessions.value = [];

  const handler = useMessageHandler({
    sessions,
    bridges: new Map(),
    activeSessionId,
    savedSessions,
    saveSessionsToStore: async () => {},
    cancelPermission: () => {},
    resolveToolAppUi: (toolName) =>
      toolName === 'Read file'
        ? {
            resourceUri: 'ui://filesystem/read-file.html',
            csp: "default-src 'self'",
            permissions: ['camera'],
          }
        : null,
  });

  return { handler, session, store, localId };
}

describe('useWorkflowTree live integration', () => {
  beforeAll(async ({}, suite) => {
    savedLocation = window.location;
    setWindowLocation(`${SERVER_ORIGIN}/workflow-viewer`);
    const probe = await probeMemoryApi();

    if (!probe.available) {
      console.warn(
        `[composableIntegration] Skipping live workflow tree tests: ${probe.reason ?? 'memory API is unavailable'}`,
      );
      skipSuite(suite as MutableSuite);
      return;
    }

    workflowUri = await findWorkflowUri();
    if (!workflowUri) {
      console.warn('[composableIntegration] Skipping live workflow tree tests: no workflow memories found on the server.');
      skipSuite(suite as MutableSuite);
    }
  });

  beforeEach(() => {
    setWindowLocation(`${SERVER_ORIGIN}/workflow-viewer`);
  });

  afterAll(() => {
    Object.defineProperty(window, 'location', {
      value: savedLocation,
      writable: true,
      configurable: true,
    });
  });

  it('parses a real workflow execution tree from memory API data', async () => {
    expect(workflowUri).toEqual(expect.any(String));

    const { loadTree, tree, loading, error } = useWorkflowTree();
    const result = await loadTree(workflowUri!);

    expect(loading.value).toBe(false);
    expect(error.value).toBeNull();
    expect(tree.value).toEqual(result);
    expect(result.type).toBe('workflow');
    if (result.type !== 'workflow') {
      throw new Error('expected a workflow tree root');
    }

    expect(result.data.uri).toBe(workflowUri);
    expect(result.data.workflowId.trim().length).toBeGreaterThan(0);
    expect(result.data.workflowName.trim().length).toBeGreaterThan(0);
    expect(result.data.steps.length).toBeGreaterThan(0);
    expect(result.data.steps[0]?.stepNumber).toBe(1);
    expect(result.data.steps.every((step, index) => step.stepNumber === index + 1)).toBe(true);

    for (const child of result.children) {
      if (child.type === 'thinking') {
        expect(child.data.uri).toContain('/thinking/sequential/');
        expect(Array.isArray(child.data.thoughts)).toBe(true);
      } else {
        expect(child.data.uri).toContain('/thinking/workflow/');
        expect(child.data.steps.length).toBeGreaterThan(0);
      }
    }
  });
});

describe('useMessageHandler integration', () => {
  beforeAll(() => {
    savedLocation = window.location;
  });

  afterAll(() => {
    Object.defineProperty(window, 'location', {
      value: savedLocation,
      writable: true,
      configurable: true,
    });
  });

  it('appends agent message chunks into the active session state', async () => {
    const { handler, store, session, localId } = createHandlerHarness();

    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'agent_message_chunk',
        content: { type: 'text', text: 'Hello ' },
      },
    } as SessionNotification);

    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'agent_message_chunk',
        content: { type: 'text', text: 'world!' },
      },
    } as SessionNotification);

    await nextTick();

    expect(session.messages).toHaveLength(1);
    expect(session.messages[0]?.role).toBe('assistant');
    expect(session.messages[0]?.content).toBe('Hello world!');
    expect(store.messageList).toHaveLength(1);
    expect(store.messageList[0]?.content).toBe('Hello world!');
  });

  it('creates and updates a tool-only assistant turn using real session state', async () => {
    const { handler, store, session, localId } = createHandlerHarness();

    session.messages.push({
      id: 'user-msg-1',
      role: 'user',
      content: 'Do something',
      timestamp: Date.now(),
    });

    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'tool_call',
        toolCallId: 'tc-1',
        title: 'Read file',
        kind: 'read',
        status: 'pending',
      },
    } as SessionNotification);

    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'tool_call_update',
        toolCallId: 'tc-1',
        title: 'Read file',
        status: 'completed',
        content: [{ type: 'content', content: { type: 'text', text: 'completed' } }],
        rawOutput: { ok: true },
      },
    } as unknown as SessionNotification);

    await nextTick();

    expect(store.messageList).toHaveLength(2);
    const assistantMessage = store.messageList[1];
    expect(assistantMessage?.role).toBe('assistant');
    expect(assistantMessage?.content).toBe('');
    expect(assistantMessage?.toolCalls).toHaveLength(1);
    expect(assistantMessage?.toolCalls?.[0]).toMatchObject({
      toolCallId: 'tc-1',
      title: 'Read file',
      status: 'completed',
      kind: 'read',
      appUi: {
        resourceUri: 'ui://filesystem/read-file.html',
        csp: "default-src 'self'",
        permissions: ['camera'],
      },
      rawOutput: { ok: true },
    });
    expect(assistantMessage?.toolCalls?.[0]?.content).toEqual([
      { type: 'content', content: { type: 'text', text: 'completed' } },
    ]);
    expect(session.toolCalls.get('tc-1')).toMatchObject({
      toolCallId: 'tc-1',
      status: 'completed',
    });
  });
});
