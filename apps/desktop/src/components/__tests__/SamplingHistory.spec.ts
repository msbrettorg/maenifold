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
  toolCallCount?: number;
  toolLoopBudget?: number;
};

const samplingHistoryStore = reactive({
  entries: [] as SamplingHistoryEntry[],
  init: vi.fn(),
  refresh: vi.fn(),
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
    status: 'streaming',
    decision: 'allow_once',
    createdAt: '2026-05-13T12:00:00.0000000+00:00',
    tokenUsage: { inputTokens: 12, outputTokens: 34 },
    requestPayload: '{"intent":"read config"}',
    responsePayload: '{"status":"candidates"}',
    ...overrides,
  };
}

async function mountSubject(entries: SamplingHistoryEntry[]) {
  samplingHistoryStore.entries = entries;
  const { default: SamplingHistory } = await import('../SamplingHistory.vue');
  return mount(SamplingHistory, {
    global: {
      stubs: {
        Badge: { template: '<span v-bind="$attrs"><slot /></span>' },
        Button: { template: '<button type="button" v-bind="$attrs"><slot /></button>' },
        Table: { template: '<table><slot /></table>' },
        TableBody: { template: '<tbody><slot /></tbody>' },
        TableCell: { template: '<td><slot /></td>' },
        TableHead: { template: '<th><slot /></th>' },
        TableHeader: { template: '<thead><slot /></thead>' },
        TableRow: { template: '<tr><slot /></tr>' },
      },
    },
  });
}

describe('SamplingHistory tool loop badges', () => {
  beforeEach(() => {
    samplingHistoryStore.entries = [];
    samplingHistoryStore.init.mockReset();
    samplingHistoryStore.refresh.mockReset();
  });

  it('renders tool call count and budget', async () => {
    const wrapper = await mountSubject([makeEntry({ toolCallCount: 5, toolLoopBudget: 10 })]);

    expect(wrapper.get('[data-testid="tool-budget-badge"]').text()).toContain('Tool calls: 5/10');
    wrapper.unmount();
  });

  it('uses amber styling when tool call count is at least eighty percent of budget', async () => {
    const wrapper = await mountSubject([makeEntry({ toolCallCount: 8, toolLoopBudget: 10 })]);

    expect(wrapper.get('[data-testid="tool-budget-badge"]').classes()).toContain('bg-warning/10');
    expect(wrapper.get('[data-testid="tool-budget-badge"]').classes()).toContain('text-warning');
    wrapper.unmount();
  });

  it('uses destructive styling when status is budgetExceeded', async () => {
    const wrapper = await mountSubject([
      makeEntry({ status: 'budgetExceeded', toolCallCount: 10, toolLoopBudget: 10 }),
    ]);

    expect(wrapper.get('[data-testid="tool-budget-badge"]').classes()).toContain('bg-destructive/10');
    expect(wrapper.get('[data-testid="tool-budget-badge"]').classes()).toContain('text-destructive');
    wrapper.unmount();
  });
});
