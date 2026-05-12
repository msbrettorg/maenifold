// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { ref } from 'vue';
import { afterEach, describe, expect, it, vi } from 'vitest';
import GateTaskTable from '../GateTaskTable.vue';
import { useGateTaskFiltering } from '../../composables/useGateTaskFiltering';
import { createGateSnapshot, type GateSnapshot, type GateTask } from '../../lib/gate';

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

  it('emits Last Updated sort requests and exposes aria-sort state', async () => {
    const wrapper = mount(GateTaskTable, {
      props: {
        tasks: [task],
        selectedTaskKey: '',
        sortColumn: 'lastUpdated',
        sortDirection: 'desc',
      },
    });

    const lastUpdatedHeader = wrapper.findAll('th')
      .find((header) => header.text().includes('Last Updated'));
    const lastUpdatedButton = wrapper.findAll('button')
      .find((button) => button.text().includes('Last Updated'));

    expect(lastUpdatedHeader?.attributes('aria-sort')).toBe('descending');
    await lastUpdatedButton?.trigger('click');
    expect(wrapper.emitted('sort')).toEqual([['lastUpdated']]);
  });

  it('cycles Last Updated sort through descending, ascending, then default', () => {
    const olderTask = {
      ...task,
      taskId: 'T-238.48',
      canonicalTaskId: 'T-238.48',
      location: 'todo',
      lastUpdatedAt: '2026-05-01T10:00:00.000Z',
    };
    const newerTask = {
      ...task,
      taskId: 'T-238.49',
      canonicalTaskId: 'T-238.49',
      location: 'todo',
      lastUpdatedAt: '2026-05-01T11:00:00.000Z',
    };
    const emptyTask = {
      ...task,
      taskId: 'T-238.50',
      canonicalTaskId: 'T-238.50',
      location: 'todo',
      lastUpdatedAt: null,
    };
    const snapshot = ref<GateSnapshot | null>(
      createGateSnapshot('/repo', [emptyTask, newerTask, olderTask]),
    );
    const filtering = useGateTaskFiltering(snapshot);

    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-238.48',
      'T-238.49',
      'T-238.50',
    ]);

    filtering.toggleSort('lastUpdated');
    expect(filtering.sortColumn.value).toBe('lastUpdated');
    expect(filtering.sortDirection.value).toBe('desc');
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-238.49',
      'T-238.48',
      'T-238.50',
    ]);

    filtering.toggleSort('lastUpdated');
    expect(filtering.sortDirection.value).toBe('asc');
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-238.50',
      'T-238.48',
      'T-238.49',
    ]);

    filtering.toggleSort('lastUpdated');
    expect(filtering.sortColumn.value).toBe('id');
    expect(filtering.sortDirection.value).toBe('asc');
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-238.48',
      'T-238.49',
      'T-238.50',
    ]);
  });
});
