// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import PipelineSidebarItem from '../PipelineSidebarItem.vue';
import type { PipelineListItem } from '../../lib/pipeline-types';

function pipelineItem(overrides: Partial<PipelineListItem> = {}): PipelineListItem {
  return {
    pipelineId: 'research-pipeline',
    status: 'completed',
    nodeCount: 4,
    completedCount: 3,
    ...overrides,
  };
}

function mountItem(item: PipelineListItem) {
  return mount(PipelineSidebarItem, {
    props: {
      item,
      active: false,
    },
  });
}

describe('PipelineSidebarItem', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-05-01T12:00:00.000Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders pipeline description and relative last run timestamp', () => {
    const wrapper = mountItem(pipelineItem({
      description: 'Investigate a topic and summarize the findings for review.',
      lastRunAt: '2026-05-01T11:00:00.000Z',
    }));

    expect(wrapper.text()).toContain('Investigate a topic and summarize the findings for review.');
    expect(wrapper.text()).toContain('Last run: 1h ago');
    expect(wrapper.get('time').attributes('datetime')).toBe('2026-05-01T11:00:00.000Z');
  });

  it('uses the most recent run timestamp before update fallbacks', () => {
    const wrapper = mountItem(pipelineItem({
      runs: [
        { startedAt: '2026-05-01T08:00:00.000Z' },
        { completedAt: '2026-05-01T11:30:00.000Z' },
      ],
      updatedAt: '2026-05-01T10:00:00.000Z',
    }));

    expect(wrapper.text()).toContain('Last run: 30m ago');
    expect(wrapper.get('time').attributes('datetime')).toBe('2026-05-01T11:30:00.000Z');
  });

  it('falls back gracefully when metadata is absent', () => {
    const wrapper = mountItem(pipelineItem());

    expect(wrapper.text()).toContain('—');
    expect(wrapper.find('time').exists()).toBe(false);
  });
});
