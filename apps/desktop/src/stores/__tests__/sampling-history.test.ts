// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

type TokenUsage = {
  inputTokens: number;
  outputTokens: number;
};

type SamplingHistoryEntry = {
  samplingRequestId: string;
  appSessionId: string;
  acpSessionId: string | null;
  serverName: string;
  model: string;
  runtimeKind: string;
  roleName: string;
  status: string;
  decision: string;
  createdAt: string;
  tokenUsage: TokenUsage | null;
};

type SamplingHistoryStore = {
  entries: SamplingHistoryEntry[];
  init(): Promise<void>;
  refresh(): Promise<void>;
};

const readResource = vi.fn();
const subscribeResource = vi.fn().mockResolvedValue(() => Promise.resolve());

vi.mock('../../composables/useMcpResources', () => ({
  useMcpResources: () => ({
    readResource,
    subscribeResource,
  }),
}));

function makeEntry(index: number, overrides: Partial<SamplingHistoryEntry> = {}): SamplingHistoryEntry {
  return {
    samplingRequestId: `00000000-0000-4000-8000-${index.toString().padStart(12, '0')}`,
    appSessionId: `10000000-0000-4000-8000-${index.toString().padStart(12, '0')}`,
    acpSessionId: `acp-${index}`,
    serverName: `server-${index}`,
    model: `model-${index}`,
    runtimeKind: 'chat',
    roleName: `role-${index}`,
    status: 'completed',
    decision: 'allow_once',
    createdAt: `2026-05-13T12:${index.toString().padStart(2, '0')}:00.0000000+00:00`,
    tokenUsage: { inputTokens: index, outputTokens: index + 1 },
    ...overrides,
  };
}

function mockSessions(sessions: unknown[]): void {
  readResource.mockResolvedValue({
    uri: 'sampling://sessions',
    mimeType: 'application/json',
    text: JSON.stringify({
      sessions,
      generatedAt: '2026-05-13T13:00:00.0000000+00:00',
    }),
  });
}

async function setupStore(sessions: unknown[]): Promise<SamplingHistoryStore> {
  vi.resetModules();
  setActivePinia(createPinia());
  mockSessions(sessions);
  const { useSamplingHistoryStore } = await import('../sampling-history');
  const store = useSamplingHistoryStore() as SamplingHistoryStore;
  await store.init();
  return store;
}

describe('sampling-history store', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    readResource.mockReset();
    subscribeResource.mockReset();
    subscribeResource.mockResolvedValue(() => Promise.resolve());
  });

  it('sampling-history store loads entries from sampling://sessions on init', async () => {
    const first = makeEntry(1);
    const second = makeEntry(2, { status: 'failed', decision: 'deny' });

    const store = await setupStore([first, second]);

    expect(readResource).toHaveBeenCalledWith('sampling://sessions');
    expect(store.entries).toHaveLength(2);
    expect(store.entries[0]).toEqual(first);
    expect(store.entries[1]).toEqual(second);
  });

  it('sampling-history store caps at 50 entries', async () => {
    const store = await setupStore(Array.from({ length: 100 }, (_, index) => makeEntry(index)));

    expect(store.entries).toHaveLength(50);
  });

  it('refresh action re-reads the resource', async () => {
    const store = await setupStore([makeEntry(1)]);

    await store.refresh();

    expect(readResource).toHaveBeenCalledTimes(2);
    expect(readResource).toHaveBeenNthCalledWith(1, 'sampling://sessions');
    expect(readResource).toHaveBeenNthCalledWith(2, 'sampling://sessions');
  });

  it('malformed entries are skipped gracefully', async () => {
    const valid = makeEntry(1);
    const missingStatus = { ...makeEntry(2), status: undefined };
    const missingCreatedAt = { ...makeEntry(3), createdAt: undefined };

    const store = await setupStore([missingStatus, valid, missingCreatedAt]);

    expect(store.entries).toEqual([valid]);
  });

  it('tokenUsage None state preserved', async () => {
    const entry = makeEntry(1, { tokenUsage: null });

    const store = await setupStore([entry]);

    expect(store.entries).toHaveLength(1);
    expect(store.entries[0]?.tokenUsage).toBeNull();
  });
});
