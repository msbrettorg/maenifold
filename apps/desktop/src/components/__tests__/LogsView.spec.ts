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
  requestPayload?: string;
  responsePayload?: string;
  currentDelta?: string;
};

const samplingHistoryStore = reactive({
  entries: [] as SamplingHistoryEntry[],
  start: vi.fn(),
  stop: vi.fn(),
  refresh: vi.fn(),
  cancelSamplingSession: vi.fn(),
});

vi.mock('../../stores/sampling-history', () => ({
  useSamplingHistoryStore: () => samplingHistoryStore,
}));

function entry(overrides: Partial<SamplingHistoryEntry> = {}): SamplingHistoryEntry {
  return {
    samplingRequestId: '00000000-0000-4000-8000-000000000001',
    appSessionId: '10000000-0000-4000-8000-000000000001',
    acpSessionId: 'acp-1',
    serverName: 'GatewayIntent',
    model: 'gpt-5.4',
    runtimeKind: 'clientSampling',
    roleName: 'assistant',
    status: 'streaming',
    decision: 'clientSampling',
    createdAt: '2026-05-13T12:00:00.0000000+00:00',
    tokenUsage: null,
    requestPayload: '{"intent":"resolve tool"}',
    responsePayload: '{"tool":"GatewayExecute"}',
    currentDelta: 'streaming tool choice',
    ...overrides,
  };
}

async function mountSubject(entries: SamplingHistoryEntry[]) {
  samplingHistoryStore.entries = entries;
  const { default: LogsView } = await import('../LogsView.vue');
  return mount(LogsView);
}

describe('LogsView', () => {
  beforeEach(() => {
    samplingHistoryStore.entries = [];
    samplingHistoryStore.start.mockReset();
    samplingHistoryStore.stop.mockReset();
    samplingHistoryStore.refresh.mockReset();
    samplingHistoryStore.cancelSamplingSession.mockReset();
  });

  it('renders the unified inference logs surface', async () => {
    const wrapper = await mountSubject([
      entry(),
      entry({ samplingRequestId: '00000000-0000-4000-8000-000000000002', status: 'failed' }),
    ]);

    expect(samplingHistoryStore.start).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('Logs');
    expect(wrapper.text()).toContain('Unified visibility');
    expect(wrapper.text()).toContain('2 inference events');
    expect(wrapper.text()).toContain('1 active');
    expect(wrapper.text()).toContain('1 failed');
    expect(wrapper.text()).toContain('Client sampling');
    expect(wrapper.text()).toContain('GatewayIntent');
    expect(wrapper.text()).toContain('resolve tool');
    expect(wrapper.text()).toContain('Live stream');
    expect(wrapper.text()).toContain('streaming tool choice');
    wrapper.unmount();
    expect(samplingHistoryStore.stop).toHaveBeenCalledTimes(1);
  });

  it('shows distinct GatewayIntent event types', async () => {
    const wrapper = await mountSubject([
      entry({ decision: 'clientSampling', runtimeKind: 'clientSampling' }),
      entry({
        samplingRequestId: '00000000-0000-4000-8000-000000000003',
        appSessionId: '10000000-0000-4000-8000-000000000003',
        decision: 'inferencing',
        runtimeKind: 'claude-code',
      }),
    ]);

    expect(wrapper.text()).toContain('Client sampling');
    expect(wrapper.text()).toContain('Inferencing');
    expect(wrapper.text()).toContain('claude-code');
    wrapper.unmount();
  });

  it('refreshes the sampling history store', async () => {
    const wrapper = await mountSubject([entry()]);

    await wrapper.get('button').trigger('click');

    expect(samplingHistoryStore.refresh).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });

  it('can stop an active inference event', async () => {
    const wrapper = await mountSubject([entry()]);

    await wrapper.get('button[aria-label="Stop sampling session 00000000-0000-4000-8000-000000000001"]').trigger('click');

    expect(samplingHistoryStore.cancelSamplingSession).toHaveBeenCalledWith('00000000-0000-4000-8000-000000000001');
    wrapper.unmount();
  });
});
