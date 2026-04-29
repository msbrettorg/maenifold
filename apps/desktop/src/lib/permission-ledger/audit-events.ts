import { ref } from 'vue';
import type { LastAutoPermissionDecision } from './ledger';

export type PermissionGhostNotification = LastAutoPermissionDecision & {
  id: string;
  message: string;
};

export const permissionGhostNotifications = ref<PermissionGhostNotification[]>([]);
export const autoPermissionStatuses = ref<Record<string, 'auto-allowed' | 'auto-denied'>>({});

export function pushPermissionGhostNotification(event: LastAutoPermissionDecision): void {
  const notification: PermissionGhostNotification = {
    ...event,
    id: `${event.permissionId ?? 'mode'}-${Date.now()}`,
    message: `Auto-${event.decision === 'allow' ? 'allowed' : 'denied'} ${event.toolKind}${event.canonicalTarget ? ` of ${event.canonicalTarget}` : ''}`,
  };
  permissionGhostNotifications.value = [...permissionGhostNotifications.value, notification];
  globalThis.setTimeout(() => {
    permissionGhostNotifications.value = permissionGhostNotifications.value.filter(
      (candidate) => candidate.id !== notification.id
    );
  }, 3_000);
}

export function markAutoPermissionStatus(toolCallId: string, decision: 'allow' | 'reject'): void {
  autoPermissionStatuses.value = {
    ...autoPermissionStatuses.value,
    [toolCallId]: decision === 'allow' ? 'auto-allowed' : 'auto-denied',
  };
}
