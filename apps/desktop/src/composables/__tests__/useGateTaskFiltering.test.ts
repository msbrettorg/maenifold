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
