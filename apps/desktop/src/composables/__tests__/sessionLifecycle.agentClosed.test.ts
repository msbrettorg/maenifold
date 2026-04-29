// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reactive, ref } from 'vue';
import type { AcpClientBridge } from '../../lib/acp-client';
import { createEmptySessionState } from '../../lib/types';
import type { SavedSession, SessionState, ToolCallInfo } from '../../lib/types';
import { createMockBridge } from '../../test/setup';
import { useSessionLifecycle } from '../useSessionLifecycle';

function createSavedSession(): SavedSession {
  return {
    id: 'saved-1',
    agentName: 'OpenCode',
    sessionId: 'remote-session-1',
    title: 'Saved Session',
    lastUpdated: Date.now(),
    cwd: '/repo',
    supportsLoadSession: true,
  };
}

function createToolCall(toolCallId: string, status: ToolCallInfo['status']): ToolCallInfo {
  return {
    toolCallId,
    title: toolCallId,
    kind: 'bash',
    status,
  };
}

function buildLifecycleHarness(session: SessionState) {
  const sessions = ref(new Map([[session.sessionId, session]]));
  const activeSessionId = ref<string | null>(session.sessionId);
  const savedSessions = ref<SavedSession[]>([]);
  const bridge = createMockBridge(session.agentId ?? 'runtime-agent');
  const bridges = new Map<string, AcpClientBridge>([
    [session.sessionId, bridge as unknown as AcpClientBridge],
  ]);

  useSessionLifecycle({
    sessions,
    bridges,
    activeSessionId,
    savedSessions,
    saveSessionsToStore: async () => {},
    mergeSessionsFromAgent: async () => {},
    handleSessionUpdate: () => {},
    promptForAuthMethod: async () => null,
    cancelAuthResolver: () => {},
    getAppVersion: () => '0.1.0',
    clearConnectionError: () => {},
    cancelPendingToolCalls: (target) => {
      for (const toolCall of target.toolCalls.values()) {
        if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
          toolCall.status = 'cancelled';
        }
      }

      for (const message of target.messages) {
        for (const toolCall of message.toolCalls ?? []) {
          if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
            toolCall.status = 'cancelled';
          }
        }
      }
    },
    pushSystemMessage: (target, content) => {
      target.messages.push({
        id: crypto.randomUUID(),
        role: 'system',
        content,
        timestamp: Date.now(),
      });
    },
  });

  async function emitAgentClosed() {
    bridge.markTransportClosed('socket closed');
    await vi.advanceTimersByTimeAsync(600);
  }

  return {
    bridge,
    emitAgentClosed,
    session,
  };
}

describe('useSessionLifecycle agent closed handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  it('tears down a connected session immediately when the agent exits', async () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    session.agentId = 'runtime-agent-1';
    session.session = createSavedSession();
    session.isConnected = true;
    session.isLoading = true;
    session.currentPlan = [{ content: 'Run tool', priority: 'high', status: 'in_progress' }];
    session.pendingPermissions = [
      {
        sessionId: 'remote-session-1',
        toolCall: createToolCall('tool-pending', 'pending'),
        options: [],
      },
    ];
    session.pendingAuthMethods = [{ id: 'browser', name: 'Browser' }];
    session.pendingAuthAgentName = 'OpenCode';
    session.configOptions = [
      {
        id: 'temperature',
        name: 'Temperature',
        type: 'select',
        currentValue: '0',
        options: [],
      },
    ];
    session.hasConfigOptions = true;
    session.toolCalls.set('tool-pending', createToolCall('tool-pending', 'pending'));
    session.toolCalls.set('tool-running', createToolCall('tool-running', 'in_progress'));
    session.toolCalls.set('tool-done', createToolCall('tool-done', 'completed'));
    session.messages.push({
      id: 'assistant-1',
      role: 'assistant',
      content: 'Working',
      timestamp: Date.now(),
      toolCalls: [
        createToolCall('tool-pending', 'pending'),
        createToolCall('tool-running', 'in_progress'),
        createToolCall('tool-done', 'completed'),
      ],
    });

    const { bridge, emitAgentClosed } = buildLifecycleHarness(session);

    await emitAgentClosed();

    expect(bridge.disconnect).toHaveBeenCalledOnce();
    expect(session.agentId).toBeNull();
    expect(session.isConnected).toBe(false);
    expect(session.isConnecting).toBe(false);
    expect(session.isLoading).toBe(false);
    expect(session.error).toContain('exited');
    expect(session.pendingPermissions).toEqual([]);
    expect(session.pendingAuthMethods).toEqual([]);
    expect(session.pendingAuthAgentName).toBe('');
    expect(session.currentPlan).toBeNull();
    expect(session.configOptions).toEqual([]);
    expect(session.hasConfigOptions).toBe(false);
    expect(session.toolCalls.get('tool-pending')?.status).toBe('cancelled');
    expect(session.toolCalls.get('tool-running')?.status).toBe('cancelled');
    expect(session.toolCalls.get('tool-done')?.status).toBe('completed');
    expect(session.messages[session.messages.length - 1]?.role).toBe('system');
    expect(session.messages[session.messages.length - 1]?.content).toContain('disconnected');
    expect(session.messages[0].toolCalls?.[0]?.status).toBe('cancelled');
    expect(session.messages[0].toolCalls?.[1]?.status).toBe('cancelled');
    expect(session.messages[0].toolCalls?.[2]?.status).toBe('completed');
  });

  it('marks an in-progress startup as failed when the agent exits early', async () => {
    const session = reactive(createEmptySessionState('local-2', 'OpenCode', '/repo'));
    session.agentId = 'runtime-agent-2';
    session.isConnecting = true;
    session.isLoading = true;
    session.startupPhase = 'connecting';

    const { bridge, emitAgentClosed } = buildLifecycleHarness(session);

    await emitAgentClosed();

    expect(bridge.disconnect).toHaveBeenCalledOnce();
    expect(session.agentId).toBeNull();
    expect(session.isConnecting).toBe(false);
    expect(session.isLoading).toBe(false);
    expect(session.startupPhase).toBe('agent_closed');
    expect(session.error).toContain('exited');
    expect(session.messages[session.messages.length - 1]?.role).toBe('system');
  });
});
