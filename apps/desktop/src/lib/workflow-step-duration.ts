import type { WorkflowStepTimings } from '../stores/workflow';

export function formatDurationMs(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  if (ms < 1000) return `${Math.max(1, Math.round(ms))}ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${Number(seconds.toFixed(1))}s`;
  return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`;
}

export function stepDurationLabel(
  stepTimings: WorkflowStepTimings | undefined,
  stepNumber: number,
): string {
  const timing = stepTimings?.[String(stepNumber)];
  if (!timing?.started_at) return '';
  if (!timing.ended_at) return 'In progress';
  const ms = new Date(timing.ended_at).getTime() - new Date(timing.started_at).getTime();
  const duration = formatDurationMs(ms);
  return duration === '—' ? '' : `Duration: ${duration}`;
}
