// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import PipelineRunHistoryPanel from '../PipelineRunHistoryPanel.vue';
import type { PipelineRunSummary } from '../../lib/pipeline-types';

function run(overrides: Partial<PipelineRunSummary> & { nodeId: string }): PipelineRunSummary {
  return {
    nodeId: overrides.nodeId,
    startedAt: overrides.startedAt ?? null,
    completedAt: overrides.completedAt ?? null,
    status: overrides.status ?? 'pending',
    outcome: overrides.outcome ?? null,
  };
}

// 2026-05-01T12:00:00Z = 1746100800000ms
const NOW_MS = 1746100800000;
// 2026-05-01T10:00:00Z = 2h before
const TWO_HOURS_BEFORE = NOW_MS - 2 * 60 * 60 * 1000;
// 2026-05-01T10:00:05Z = 2h before + 5s
const FIVE_SECS_AFTER = TWO_HOURS_BEFORE + 5000;

describe('PipelineRunHistoryPanel', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(NOW_MS));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows 3 rows with status chips and outcome badges for 3 completed nodes', () => {
    const runs: PipelineRunSummary[] = [
      run({ nodeId: 'plan-node', startedAt: TWO_HOURS_BEFORE, completedAt: FIVE_SECS_AFTER, status: 'completed', outcome: 'succeeded' }),
      run({ nodeId: 'exec-node', startedAt: TWO_HOURS_BEFORE, completedAt: FIVE_SECS_AFTER, status: 'completed', outcome: 'succeeded' }),
      run({ nodeId: 'review-node', startedAt: TWO_HOURS_BEFORE, completedAt: FIVE_SECS_AFTER, status: 'completed', outcome: 'failed' }),
    ];

    const wrapper = mount(PipelineRunHistoryPanel, { props: { runs } });

    const rows = wrapper.findAll('[data-testid^="run-row-"]');
    expect(rows).toHaveLength(3);

    const chips = wrapper.findAll('[data-testid="run-status-chip"]');
    chips.forEach((chip) => {
      expect(chip.attributes('data-status')).toBe('completed');
    });

    const outcomes = wrapper.findAll('[data-testid="run-outcome-badge"]');
    expect(outcomes).toHaveLength(3);
    expect(outcomes[0].attributes('data-outcome')).toBe('succeeded');
    expect(outcomes[2].attributes('data-outcome')).toBe('failed');
  });

  it('renders correct chip classes for mixed statuses', () => {
    const runs: PipelineRunSummary[] = [
      run({ nodeId: 'n1', status: 'active', startedAt: TWO_HOURS_BEFORE }),
      run({ nodeId: 'n2', status: 'abandoned' }),
      run({ nodeId: 'n3', status: 'pending' }),
      run({ nodeId: 'n4', status: 'stuck' }),
    ];

    const wrapper = mount(PipelineRunHistoryPanel, { props: { runs } });

    const chips = wrapper.findAll('[data-testid="run-status-chip"]');
    expect(chips).toHaveLength(4);

    expect(chips[0].classes()).toContain('text-status-active');
    expect(chips[1].classes()).toContain('text-status-abandoned');
    expect(chips[2].classes()).toContain('text-status-pending');
    expect(chips[3].classes()).toContain('text-status-stuck');
  });

  it('shows empty state when runs array is empty', () => {
    const wrapper = mount(PipelineRunHistoryPanel, { props: { runs: [] } });

    expect(wrapper.find('[data-testid="run-history-empty"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="run-history-empty"]').text()).toBe('No runs recorded yet');
    expect(wrapper.find('[data-testid="run-history-list"]').exists()).toBe(false);
  });

  it('shows dash placeholder for null startedAt on a pending node', () => {
    const runs: PipelineRunSummary[] = [
      run({ nodeId: 'waiting-node', startedAt: null, status: 'pending' }),
    ];

    const wrapper = mount(PipelineRunHistoryPanel, { props: { runs } });

    expect(wrapper.find('[data-testid="run-started"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="run-started-empty"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="run-started-empty"]').text()).toBe('—');
  });
});
