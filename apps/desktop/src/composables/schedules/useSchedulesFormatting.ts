import type { ScheduleJob } from '@/stores/schedules';

type SchedulePermissionInfo = {
  auto_grant_permissions: boolean;
  allowed_tools?: string[];
};

export type PermissionBadgeVariant = 'default' | 'secondary' | 'destructive';

const timestampFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
});

export function permissionLabel(schedule: SchedulePermissionInfo): string {
  if (schedule.auto_grant_permissions) return 'Auto-grant on';
  if (schedule.allowed_tools?.length) return 'Approval required · allowlist set';
  return 'Approval required';
}

export function permissionVariant(schedule: SchedulePermissionInfo): PermissionBadgeVariant {
  return schedule.auto_grant_permissions ? 'destructive' : 'secondary';
}

export function allowedToolsLabel(schedule: SchedulePermissionInfo): string {
  return schedule.allowed_tools?.length ? schedule.allowed_tools.join(', ') : 'No tool allowlist';
}

export function scheduleLabel(job: ScheduleJob): string {
  return job.schedule_human?.trim() || job.schedule;
}

export function statusFor(job: ScheduleJob): { label: string; variant: 'default' | 'secondary' | 'destructive' } {
  if (!job.enabled) return { label: 'Paused', variant: 'secondary' };
  if (job.last_run?.status === 'failed' || job.last_run?.status === 'timeout') {
    return { label: 'Failed', variant: 'destructive' };
  }
  return { label: 'Active', variant: 'default' };
}

export function formatLastRun(timestamp?: string | null): string {
  if (!timestamp) return 'Never';
  const parsed = new Date(timestamp);
  return Number.isNaN(parsed.getTime()) ? timestamp : timestampFormatter.format(parsed);
}

export function formatPausedAt(timestamp?: string): string {
  if (!timestamp) return 'an unknown time';
  const parsed = new Date(timestamp);
  return Number.isNaN(parsed.getTime()) ? timestamp : timestampFormatter.format(parsed);
}
