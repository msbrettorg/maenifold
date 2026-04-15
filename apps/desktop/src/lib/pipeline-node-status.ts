import type { NodeStatus, PipelineStatus } from './pipeline-types';

export type PipelineNodeStatusAppearance =
  | 'pending'
  | 'active'
  | 'completed'
  | 'abandoned'
  | 'stuck'
  | 'cancelled';

export function normalizeNodeStatusAppearance(
  status?: NodeStatus | PipelineStatus | 'running',
): PipelineNodeStatusAppearance {
  switch (status) {
    case 'active':
    case 'running':
    case 'retrying':
      return 'active';
    case 'completed':
      return 'completed';
    case 'abandoned':
      return 'abandoned';
    case 'stuck':
      return 'stuck';
    case 'cancelled':
      return 'cancelled';
    case 'pending':
    case 'ready':
    case 'paused':
    default:
      return 'pending';
  }
}

export function getNodeStatusBorderColor(
  appearance: PipelineNodeStatusAppearance,
): string {
  switch (appearance) {
    case 'active':
      return 'var(--color-status-active)';
    case 'completed':
      return 'var(--color-status-completed)';
    case 'abandoned':
      return 'var(--color-status-abandoned)';
    case 'stuck':
      return 'var(--color-status-active)';
    case 'cancelled':
      return 'var(--color-status-pending)';
    case 'pending':
    default:
      return 'var(--color-status-pending)';
  }
}

export function getNodeShellClassName(
  status?: NodeStatus | PipelineStatus | 'running',
): string {
  const appearance = normalizeNodeStatusAppearance(status);
  return `pipeline-node-shell pipeline-node-shell--${appearance}`;
}

export function getNodeShellStyle(
  _status?: NodeStatus | PipelineStatus | 'running',
): Record<string, string> {
  return {};
}
