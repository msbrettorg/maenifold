// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const readResource = vi.fn();
const subscribeResource = vi.fn();
const connect = vi.fn();
const callTool = vi.fn();

vi.mock('../../composables/useMcpResources', () => ({
  useMcpResources: () => ({
    readResource,
    subscribeResource,
  }),
}));

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => ({
    connect,
    callTool,
  }),
}));

vi.mock('../../lib/api-base', () => ({
  getApiBase: () => 'http://127.0.0.1:8080',
}));

class MockEventSource {
  static instances: MockEventSource[] = [];
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  close = vi.fn();

  constructor(public readonly url: string) {
    MockEventSource.instances.push(this);
  }
}

function makeEntry(overrides: Record<string, unknown> = {}) {
  return {
    samplingRequestId: '00000000-0000-4000-8000-000000000001',
    appSessionId: '10000000-0000-4000-8000-000000000001',
    acpSessionId: 'acp-1',
    serverName: 'github',
    model: 'gpt-5.4',
    runtimeKind: 'copilot',
    roleName: 'copilot',
    status: 'streaming',
    decision: 'allow_once',
    createdAt: '2026-05-13T12:00:00.000Z',
    tokenUsage: null,
    ...overrides,
  };
}

function mockSessions(sessions: unknown[]) {
  readResource.mockResolvedValue({
    uri: 'sampling://sessions',
    text: JSON.stringify({ sessions }),
  });
}

async function setupStore(sessions: unknown[]) {
  setActivePinia(createPinia());
  mockSessions(sessions);
  const { useSamplingHistoryStore } = await import('../sampling-history');
  const store = useSamplingHistoryStore();
  await store.init();
  return store;
}

describe('sampling-history live controls', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    readResource.mockReset();
    subscribeResource.mockReset();
    connect.mockReset();
    callTool.mockReset();
    subscribeResource.mockResolvedValue(vi.fn());
    connect.mockResolvedValue(undefined);
    callTool.mockResolvedValue({});
    MockEventSource.instances = [];
    vi.stubGlobal('EventSource', MockEventSource);
  });

  it('cancelSamplingSession invokes gateway with samplingRequestId', async () => {
    const entry = makeEntry();
    const store = await setupStore([entry]);

    await store.cancelSamplingSession(entry.samplingRequestId as string);

    expect(connect).toHaveBeenCalled();
    expect(callTool).toHaveBeenCalledWith('CancelSamplingSession', {
      samplingRequestId: entry.samplingRequestId,
    });
  });

  it('SSE delta updates store.entries[id].currentDelta', async () => {
    const entry = makeEntry();
    const store = await setupStore([entry]);
    const source = MockEventSource.instances[0];

    expect(source?.url).toBe('http://127.0.0.1:8080/events/sampling/10000000-0000-4000-8000-000000000001');
    source?.onmessage?.({
      data: JSON.stringify({
        type: 'acpDelta',
        appSessionId: entry.appSessionId,
        delta: 'hello',
      }),
    } as MessageEvent);

    expect(store.entries[0]?.currentDelta).toBe('hello');
  });
});
