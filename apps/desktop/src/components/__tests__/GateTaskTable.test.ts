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
    filtering.readinessFilter.value = 'all';

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
    expect(filtering.sortColumn.value).toBe('default');
    expect(filtering.sortDirection.value).toBe('asc');
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-238.48',
      'T-238.49',
      'T-238.50',
    ]);
  });
});

describe('GateStageChip', () => {
  const cases = [
    { state: 'pending',   chipClass: 'bg-status-pending/15',   label: 'Pending'   },
    { state: 'swe',       chipClass: 'bg-status-active/15',    label: 'SWE'       },
    { state: 'red-team',  chipClass: 'bg-status-stuck/15',     label: 'Red-team'  },
    { state: 'blue-team', chipClass: 'bg-status-ready/15',     label: 'Blue-team' },
    { state: 'done',      chipClass: 'bg-status-completed/15', label: 'Done'      },
    { state: 'error',     chipClass: 'bg-status-error/15',     label: 'Error'     },
  ] as const;

  cases.forEach(({ state, chipClass, label }) => {
    it(`${state}: chip has class "${chipClass}" and aria-label "Stage: ${label}"`, () => {
      const stageTask = { ...task, state, location: state === 'done' ? 'done' : 'todo' };
      const wrapper = mount(GateTaskTable, {
        props: { tasks: [stageTask], selectedTaskKey: '', sortColumn: 'id', sortDirection: 'asc' },
      });
      const chip = wrapper.find('[role="status"]');
      expect(chip.exists()).toBe(true);
      expect(chip.classes()).toContain(chipClass);
      expect(chip.text()).toBe(label);
      expect(chip.attributes('aria-label')).toBe(`Stage: ${label}`);
    });
  });
});

describe('priority column', () => {
  const priorityCases = [
    { priority: 'P0', colorClass: 'text-status-error', filled: true },
    { priority: 'P1', colorClass: 'text-status-stuck', filled: true },
    { priority: 'P2', colorClass: 'text-status-pending', filled: true },
    { priority: 'P3', colorClass: 'text-muted-foreground', filled: true },
    { priority: 'P4', colorClass: 'text-muted-foreground', filled: false },
  ] as const;

  priorityCases.forEach(({ priority, colorClass, filled }) => {
    it(`${priority}: badge has class "${colorClass}" and circle is ${filled ? 'filled' : 'outlined'}`, () => {
      const pTask = { ...task, priority };
      const wrapper = mount(GateTaskTable, {
        props: { tasks: [pTask], selectedTaskKey: '', sortColumn: 'priority', sortDirection: 'asc' },
      });
      const cell = wrapper.findAll('td').at(0)!;
      const badge = cell.find('.inline-flex');
      expect(badge.classes()).toContain(colorClass);
      expect(cell.text()).toContain(priority);
      const icon = cell.find('svg');
      expect(icon.exists()).toBe(true);
      if (filled) {
        expect(icon.classes()).toContain('fill-current');
      } else {
        expect(icon.classes()).not.toContain('fill-current');
      }
    });
  });

  it('null priority: shows "—" dash and no icon', () => {
    const pTask = { ...task, priority: null };
    const wrapper = mount(GateTaskTable, {
      props: { tasks: [pTask], selectedTaskKey: '', sortColumn: 'id', sortDirection: 'asc' },
    });
    const cell = wrapper.findAll('td').at(0)!;
    expect(cell.text()).toContain('—');
    expect(cell.find('svg').exists()).toBe(false);
  });

  it('clicking Priority header emits sort with "priority"', async () => {
    const wrapper = mount(GateTaskTable, {
      props: { tasks: [task], selectedTaskKey: '', sortColumn: 'id', sortDirection: 'asc' },
    });
    const btn = wrapper.findAll('button').find((b) => b.text().includes('Priority'));
    await btn?.trigger('click');
    expect(wrapper.emitted('sort')).toEqual([['priority']]);
  });
});
