import { ref } from 'vue';
import { describe, expect, it } from 'vitest';
import { createGateSnapshot, type GateSnapshot, type GateTask } from '../../lib/gate';
import { useGateTaskFiltering } from '../useGateTaskFiltering';

type TestGateTask = GateTask & {
  ready?: boolean;
  blockedBy?: string[];
};

const baseTask: GateTask = {
  taskId: 'T-243.59',
  canonicalTaskId: 'T-243.59',
  title: 'Gate readiness filter',
  pipeline: 'sdd',
  state: 'pending',
  stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
  attempts: 0,
  location: 'todo',
  path: '/repo/todo/T-243.59.md',
  issue: '',
};

function task(values: Partial<TestGateTask>): TestGateTask {
  const taskId = values.taskId ?? baseTask.taskId;
  return {
    ...baseTask,
    canonicalTaskId: taskId,
    path: `/repo/todo/${taskId}.md`,
    ...values,
    taskId,
  };
}

describe('useGateTaskFiltering readiness filter', () => {
  it('filters ready, blocked, and all task sets', () => {
    const snapshot = ref<GateSnapshot | null>(createGateSnapshot('/repo', [
      task({ taskId: 'T-1', title: 'ready pending task' }),
      task({ taskId: 'T-2', title: 'blocked dependency task', blockedBy: ['T-0'] }),
      task({ taskId: 'T-3', title: 'error task', state: 'error', issue: 'metadata parse failed' }),
      task({ taskId: 'T-4', title: 'pending task with error', issue: 'dependency cycle' }),
      task({ taskId: 'T-5', title: 'active task', state: 'swe' }),
      task({ taskId: 'T-6', title: 'explicitly not ready task', ready: false }),
    ]));
    const filtering = useGateTaskFiltering(snapshot);

    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-1',
      'T-2',
      'T-3',
      'T-4',
      'T-5',
      'T-6',
    ]);

    filtering.readinessFilter.value = 'ready';
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual(['T-1']);

    filtering.readinessFilter.value = 'blocked';
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-2',
      'T-3',
      'T-4',
      'T-6',
    ]);

    filtering.readinessFilter.value = 'all';
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual([
      'T-1',
      'T-2',
      'T-3',
      'T-4',
      'T-5',
      'T-6',
    ]);
  });
});

describe('useGateTaskFiltering — sprint chips', () => {
  function sprintTask(taskId: string, sprint: number | null | undefined): GateTask {
    return { ...baseTask, taskId, canonicalTaskId: taskId, sprint } as GateTask;
  }

  const taskList: GateTask[] = [
    sprintTask('T-1', 250),
    sprintTask('T-2', 251),
    sprintTask('T-3', 250),
    sprintTask('T-4', null),
    sprintTask('T-5', 249),
  ];

  it('shows all tasks when no sprint selected', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    expect(filtering.visibleTasks.value).toHaveLength(5);
  });

  it('toggleSprint adds a sprint to selectedSprints', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    filtering.toggleSprint(250);
    expect(filtering.selectedSprints.value.has(250)).toBe(true);
  });

  it('toggleSprint removes an already-selected sprint', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    filtering.toggleSprint(250);
    filtering.toggleSprint(250);
    expect(filtering.selectedSprints.value.has(250)).toBe(false);
  });

  it('filters to only matching sprint tasks (OR semantics, single sprint)', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    filtering.toggleSprint(250);
    const ids = filtering.visibleTasks.value.map((t) => t.taskId);
    expect(ids).toEqual(['T-1', 'T-3']);
  });

  it('filters to union of tasks when two sprints selected', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    filtering.toggleSprint(250);
    filtering.toggleSprint(251);
    const ids = filtering.visibleTasks.value.map((t) => t.taskId);
    expect(ids).toEqual(['T-1', 'T-2', 'T-3']);
  });

  it('sprintOptions returns top 5 sprints descending, null excluded', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    expect(filtering.sprintOptions.value).toEqual([251, 250, 249]);
  });
});

describe('useGateTaskFiltering — priority sort', () => {
  function priorityTask(taskId: string, priority: string | null): GateTask {
    return { ...baseTask, taskId, canonicalTaskId: taskId, path: `/repo/todo/${taskId}.md`, priority } as GateTask;
  }

  it('sorts P0 first ascending, null sorts last', () => {
    const tasks = [
      priorityTask('T-A', null),
      priorityTask('T-B', 'P2'),
      priorityTask('T-C', 'P0'),
      priorityTask('T-D', 'P4'),
    ];
    const snapshot = ref(createGateSnapshot('/', tasks));
    const filtering = useGateTaskFiltering(snapshot);
    filtering.toggleSort('priority');
    expect(filtering.sortColumn.value).toBe('priority');
    expect(filtering.sortDirection.value).toBe('asc');
    expect(filtering.visibleTasks.value.map((t) => t.taskId)).toEqual(['T-C', 'T-B', 'T-D', 'T-A']);
  });
});
