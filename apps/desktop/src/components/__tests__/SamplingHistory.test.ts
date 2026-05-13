// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reactive } from 'vue';

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
  tokenUsage: { inputTokens: number; outputTokens: number } | null;
};

const refresh = vi.fn();
const samplingHistoryStore = reactive({
  entries: [] as SamplingHistoryEntry[],
  refresh,
});

vi.mock('../../stores/sampling-history', () => ({
  useSamplingHistoryStore: () => samplingHistoryStore,
}));

function makeEntry(overrides: Partial<SamplingHistoryEntry> = {}): SamplingHistoryEntry {
  return {
    samplingRequestId: '00000000-0000-4000-8000-000000000001',
    appSessionId: '10000000-0000-4000-8000-000000000001',
    acpSessionId: 'acp-1',
    serverName: 'github',
    model: 'gpt-5.4',
    runtimeKind: 'chat',
    roleName: 'assistant',
    status: 'completed',
    decision: 'allow_once',
    createdAt: '2026-05-13T12:00:00.0000000+00:00',
    tokenUsage: { inputTokens: 12, outputTokens: 34 },
    ...overrides,
  };
}

async function mountSamplingHistory(entries: SamplingHistoryEntry[] = []) {
  samplingHistoryStore.entries = entries;
  const { default: SamplingHistory } = await import('../SamplingHistory.vue');
  return mount(SamplingHistory);
}

describe('SamplingHistory', () => {
  beforeEach(() => {
    refresh.mockReset();
    samplingHistoryStore.entries = [];
  });

  it('renders empty state when entries.length === 0', async () => {
    const wrapper = await mountSamplingHistory();

    expect(wrapper.text()).toContain('No sampling history');
    wrapper.unmount();
  });

  it('renders one row per entry with all 6 columns (Time, Server, Model, Decision, Status, Tokens)', async () => {
    const entries = [
      makeEntry({ serverName: 'github', model: 'gpt-5.4' }),
      makeEntry({
        samplingRequestId: '00000000-0000-4000-8000-000000000002',
        serverName: 'filesystem',
        model: 'claude-sonnet-4.6',
        decision: 'allow_always',
        status: 'streaming',
      }),
    ];

    const wrapper = await mountSamplingHistory(entries);

    for (const heading of ['Time', 'Server', 'Model', 'Decision', 'Status', 'Tokens']) {
      expect(wrapper.text()).toContain(heading);
    }
    const rows = wrapper.findAll('tbody tr');
    expect(rows).toHaveLength(2);
    expect(rows[0]?.text()).toContain('github');
    expect(rows[0]?.text()).toContain('gpt-5.4');
    expect(rows[0]?.text()).toContain('allow_once');
    expect(rows[0]?.text()).toContain('completed');
    expect(rows[0]?.findAll('td')).toHaveLength(6);
    expect(rows[0]?.findAll('td')[5]?.text()).toContain('12');
    expect(rows[0]?.findAll('td')[5]?.text()).toContain('34');
    expect(rows[1]?.text()).toContain('filesystem');
    expect(rows[1]?.text()).toContain('claude-sonnet-4.6');
    expect(rows[1]?.findAll('td')).toHaveLength(6);
    wrapper.unmount();
  });

  it('renders "not reported" for tokenUsage === null', async () => {
    const wrapper = await mountSamplingHistory([makeEntry({ tokenUsage: null })]);
    const tokenCell = wrapper.findAll('tbody tr')[0]?.findAll('td')[5];

    expect(tokenCell?.text()).toBe('not reported');
    wrapper.unmount();
  });

  it('Refresh button calls store.refresh()', async () => {
    const wrapper = await mountSamplingHistory([makeEntry()]);

    await wrapper.find('button').trigger('click');

    expect(refresh).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });
});
