import type {
  RequestPermissionRequest,
  RequestPermissionResponse,
} from '@agentclientprotocol/sdk';
import type { Ref } from 'vue';
import type { PermissionRequest as LocalPermissionRequest } from './types';

type PermissionResolver = (response: RequestPermissionResponse) => void;

export type InlinePermissionMessageSink = (message: {
  toolCallId: string;
  message: string;
  severity: 'warning';
}) => void;

export type PendingPermissionEntry = {
  request: LocalPermissionRequest;
  resolver: PermissionResolver;
};

export function buildPlanModeRejectionMessage(toolName: string, kind: string): string {
  return `Plan mode auto-rejected tool '${toolName}' (kind '${kind}' is not in the safe-kind whitelist).`;
}

export function syncPendingPermissionRequests(
  pendingPermissions: Map<string, PendingPermissionEntry>,
  pendingPermissionRequests: Ref<LocalPermissionRequest[]>
): void {
  pendingPermissionRequests.value = Array.from(pendingPermissions.values()).map(
    ({ request }) => request
  );
}

export function promptForPermission(
  params: RequestPermissionRequest,
  request: LocalPermissionRequest,
  pendingPermissions: Map<string, PendingPermissionEntry>,
  pendingPermissionRequests: Ref<LocalPermissionRequest[]>
): Promise<RequestPermissionResponse> {
  return new Promise((resolve) => {
    const key = params.toolCall.toolCallId;
    pendingPermissions.set(key, {
      request,
      resolver: resolve,
    });
    syncPendingPermissionRequests(pendingPermissions, pendingPermissionRequests);
  });
}
