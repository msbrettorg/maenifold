import type { GateTask } from './gate';

export const GATE_PIPELINES = ['sdd', 'tdd', 'gsd', 'lightweight'] as const;
export const GATE_PRIORITIES = ['P0', 'P1', 'P2', 'P3'] as const;

type PipelineId = typeof GATE_PIPELINES[number];

const FAIL_EDGES: Record<PipelineId, Record<string, string>> = {
  sdd: { swe: 'pending', red: 'pending', 'red-team': 'pending', blue: 'pending', 'blue-team': 'pending' },
  tdd: { test: 'pending', implement: 'pending', verify: 'pending', swe: 'pending', 'red-team': 'pending', 'blue-team': 'pending' },
  gsd: { research: 'pending', plan: 'pending', execute: 'pending', verify: 'execute' },
  lightweight: { swe: 'pending', blue: 'pending', 'blue-team': 'pending' },
};

function normalizedPipeline(task: GateTask): PipelineId | null {
  const pipeline = (task.pipeline ?? '').trim().toLowerCase();
  return GATE_PIPELINES.includes(pipeline as PipelineId) ? pipeline as PipelineId : null;
}

export function stageAgent(task: GateTask): string {
  return task.state.trim() || 'swe';
}

export function nextPassStage(task: GateTask): string {
  const stages = task.stages;
  const index = stages.indexOf(task.state);
  if (index >= 0 && index < stages.length - 1) return stages[index + 1];
  return 'done';
}

export function nextFailStage(task: GateTask): string {
  const pipeline = normalizedPipeline(task);
  const state = task.state.trim();
  if (pipeline && FAIL_EDGES[pipeline][state]) return FAIL_EDGES[pipeline][state];
  return 'pending';
}

export function extractSprint(taskId: string): number | null {
  const match = taskId.match(/^T-(\d+)\.\d+$/);
  return match ? Number(match[1]) : null;
}

export function nextTaskId(tasks: GateTask[]): string {
  const sprints = tasks.map((task) => extractSprint(task.taskId)).filter((value): value is number => value !== null);
  const sprint = sprints.length ? Math.max(...sprints) : 1;
  const maxInSprint = tasks.reduce((max, task) => {
    const match = task.taskId.match(new RegExp(`^T-${sprint}\\.(\\d+)$`));
    return match ? Math.max(max, Number(match[1])) : max;
  }, 0);
  return `T-${sprint}.${maxInSprint + 1}`;
}
