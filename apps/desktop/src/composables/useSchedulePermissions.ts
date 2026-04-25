type SchedulePermissionInfo = {
  auto_grant_permissions: boolean;
  allowed_tools?: string[];
};

export type PermissionBadgeVariant = 'default' | 'secondary' | 'destructive';

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

export function parseAllowedTools(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((tool) => tool.trim())
    .filter(Boolean);
}
