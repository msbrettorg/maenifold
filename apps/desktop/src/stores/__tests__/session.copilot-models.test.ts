import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick, reactive, ref } from 'vue';
import { useSessionStore } from '../session';
import { createEmptySessionState } from '../../lib/types';

const connected = ref(true);
const connect = vi.fn(async () => undefined);
const callTool = vi.fn(async () => ({
  models: [
    {
      id: 'gpt-5.5',
      capabilities: {
        supports: {
          reasoning_effort: ['low', 'medium', 'high', 'xhigh'],
        },
      },
    },
  ],
}));

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => ({
    connected,
    connect,
    callTool,
  }),
}));

describe('useSessionStore Copilot models overlay', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    connected.value = true;
    connect.mockClear();
    callTool.mockClear();
  });

  it('overlays reasoning effort for exact Copilot model ids only', async () => {
    const store = useSessionStore();
    const localId = 'copilot-session';
    const session = reactive(createEmptySessionState(localId, 'Copilot', '/test'));
    session.isConnected = true;
    session.session = {
      id: localId,
      agentName: 'Copilot',
      sessionId: 'server-session',
      title: 'Copilot Session',
      lastUpdated: Date.now(),
      cwd: '/test',
    };
    session.configOptions = [
      {
        id: 'model',
        name: 'Model',
        type: 'select',
        currentValue: 'gpt-5.5',
        options: [
          { value: 'gpt-5.5', name: 'GPT-5.5' },
          { value: 'claude-sonnet-4.6', name: 'Claude Sonnet 4.6' },
        ],
      },
      {
        id: 'reasoning_effort',
        name: 'Reasoning Effort',
        type: 'select',
        currentValue: 'medium',
        options: [
          { value: 'none', name: 'None' },
          { value: 'low', name: 'Low' },
          { value: 'medium', name: 'Medium' },
          { value: 'high', name: 'High' },
        ],
      },
    ];

    store.sessions.set(localId, session);
    store.sessions = new Map(store.sessions);
    store.activeSessionId = localId;

    await Promise.resolve();
    await nextTick();

    const reasoningOptions = store.configOptions.find((option) => option.id === 'reasoning_effort');
    expect(reasoningOptions?.options).toEqual([
      { value: 'low', name: 'Low' },
      { value: 'medium', name: 'Medium' },
      { value: 'high', name: 'High' },
      { value: 'xhigh', name: 'Extra High' },
    ]);

    session.configOptions = session.configOptions.map((option) =>
      option.id === 'model' ? { ...option, currentValue: 'claude-sonnet-4.6' } : option,
    );
    await nextTick();

    const fallbackOptions = store.configOptions.find((option) => option.id === 'reasoning_effort');
    expect(fallbackOptions?.options).toEqual([
      { value: 'none', name: 'None' },
      { value: 'low', name: 'Low' },
      { value: 'medium', name: 'Medium' },
      { value: 'high', name: 'High' },
    ]);
    expect(callTool).toHaveBeenCalledWith('GetCopilotModels', { refresh: false });
  });
});
