import { ref } from 'vue';
import { beforeEach, describe, expect, it } from 'vitest';
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
  beforeEach(() => { localStorage.clear(); });

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

    // default readiness is 'ready' — only T-1 passes isReadyTask
    expect(filtering.readinessFilter.value).toBe('ready');
    expect(filtering.visibleTasks.value.map((gateTask) => gateTask.taskId)).toEqual(['T-1']);

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
  beforeEach(() => { localStorage.clear(); });

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
    filtering.readinessFilter.value = 'all';
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
    filtering.readinessFilter.value = 'all';
    filtering.toggleSprint(250);
    const ids = filtering.visibleTasks.value.map((t) => t.taskId);
    expect(ids).toEqual(['T-1', 'T-3']);
  });

  it('filters to union of tasks when two sprints selected', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    filtering.readinessFilter.value = 'all';
    filtering.toggleSprint(250);
    filtering.toggleSprint(251);
    const ids = filtering.visibleTasks.value.map((t) => t.taskId);
    // composite sort: sprint desc → T-2(251) before T-1(250) before T-3(250)
    expect(ids).toEqual(['T-2', 'T-1', 'T-3']);
  });

  it('sprintOptions returns top 5 sprints descending, null excluded', () => {
    const snapshot = ref(createGateSnapshot('/', taskList));
    const filtering = useGateTaskFiltering(snapshot);
    expect(filtering.sprintOptions.value).toEqual([251, 250, 249]);
  });
});

describe('useGateTaskFiltering — priority sort', () => {
  beforeEach(() => { localStorage.clear(); });

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
    filtering.readinessFilter.value = 'all';
    filtering.toggleSort('priority');
    expect(filtering.sortColumn.value).toBe('priority');
    expect(filtering.sortDirection.value).toBe('asc');
    expect(filtering.visibleTasks.value.map((t) => t.taskId)).toEqual(['T-C', 'T-B', 'T-D', 'T-A']);
  });
});

describe('useGateTaskFiltering — default sort + localStorage persistence', () => {
  const SORT_KEY = 'maenifold.gateTab.sort';
  const READINESS_KEY = 'maenifold.gateTab.readiness';
  const snapshot = ref<GateSnapshot | null>(createGateSnapshot('/repo', [
    { ...baseTask, taskId: 'T-1', canonicalTaskId: 'T-1', path: '/repo/todo/T-1.md' } as GateTask,
  ]));

  beforeEach(() => { localStorage.clear(); });

  it('default state: empty localStorage → sort is default composite, readiness is ready', () => {
    const filtering = useGateTaskFiltering(snapshot);
    expect(filtering.sortColumn.value).toBe('default');
    expect(filtering.sortDirection.value).toBe('asc');
    expect(filtering.readinessFilter.value).toBe('ready');
  });

  it('override + persist: changing sortColumn writes JSON to localStorage', async () => {
    const filtering = useGateTaskFiltering(snapshot);
    filtering.toggleSort('id');
    // flush Vue watchers
    await new Promise((r) => setTimeout(r, 0));
    const stored = JSON.parse(localStorage.getItem(SORT_KEY) ?? 'null') as { column: string; direction: string } | null;
    expect(stored?.column).toBe('id');
    expect(stored?.direction).toBe('asc');
  });

  it('restore on mount: pre-populated localStorage → composable initialises from it', () => {
    localStorage.setItem(SORT_KEY, JSON.stringify({ column: 'priority', direction: 'desc' }));
    localStorage.setItem(READINESS_KEY, 'all');
    const filtering = useGateTaskFiltering(snapshot);
    expect(filtering.sortColumn.value).toBe('priority');
    expect(filtering.sortDirection.value).toBe('desc');
    expect(filtering.readinessFilter.value).toBe('all');
  });

  it('bad localStorage: malformed JSON → silently falls back to defaults', () => {
    localStorage.setItem(SORT_KEY, 'not-valid-json');
    localStorage.setItem(READINESS_KEY, 'bogus-value');
    const filtering = useGateTaskFiltering(snapshot);
    expect(filtering.sortColumn.value).toBe('default');
    expect(filtering.readinessFilter.value).toBe('ready');
  });
});
