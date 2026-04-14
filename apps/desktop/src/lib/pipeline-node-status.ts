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
      return '#3b82f6';
    case 'completed':
      return '#22c55e';
    case 'abandoned':
      return '#ef4444';
    case 'stuck':
      return '#f59e0b';
    case 'cancelled':
      return '#6b7280';
    case 'pending':
    default:
      return '#4b5563';
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
