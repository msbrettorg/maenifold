// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import GateTaskTable from '../GateTaskTable.vue';
import type { GateTask } from '../../lib/gate';

const task: GateTask = {
  taskId: 'T-238.49',
  canonicalTaskId: 'T-238.49',
  title: 'P0 Add Trouter re-registration loop to prevent silent message loss',
  pipeline: 'sdd',
  state: 'done',
  stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
  attempts: 0,
  location: 'done',
  path: '/repo/done/T-238.49.md',
  issue: '',
};

describe('GateTaskTable', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders backend task titles and pipeline labels literally', () => {
    const wrapper = mount(GateTaskTable, {
      props: {
        tasks: [task],
        selectedTaskKey: '',
        sortColumn: 'id',
        sortDirection: 'asc',
      },
    });

    const text = wrapper.text();
    expect(text).toContain('P0 Add Trouter re-registration loop to prevent silent message loss');
    expect(text).toContain('sdd');
    expect(text).not.toContain('[error] T-238.49');
    expect(text).not.toContain('sdd done');
    expect(text).not.toContain('unknown done');
  });

  it('renders last updated values and empty timestamps', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-05-01T12:00:00.000Z'));

    const wrapper = mount(GateTaskTable, {
      props: {
        tasks: [
          { ...task, lastUpdatedAt: '2026-05-01T11:55:00.000Z' },
          {
            ...task,
            taskId: 'T-238.50',
            canonicalTaskId: 'T-238.50',
            path: '/repo/done/T-238.50.md',
            lastUpdatedAt: null,
          },
        ],
        selectedTaskKey: '',
        sortColumn: 'id',
        sortDirection: 'asc',
      },
    });

    const text = wrapper.text();
    expect(text).toContain('Last Updated');
    expect(text).toContain('5m ago');
    expect(text).toContain('—');
  });
});
