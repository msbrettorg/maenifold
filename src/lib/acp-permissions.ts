import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { ref, type Ref } from 'vue';
import { normalizePermissionToolCallPayload } from './acp-meta';
import type { PermissionRequest as LocalPermissionRequest } from './types';

type PermissionResolver = (response: RequestPermissionResponse) => void;

type PendingPermissionEntry = {
  request: LocalPermissionRequest;
  resolver: PermissionResolver;
};

export class PermissionHandler {
  public pendingPermissionRequests: Ref<LocalPermissionRequest[]> = ref([]);
  private pendingPermissions = new Map<string, PendingPermissionEntry>();

  requestPermission(params: RequestPermissionRequest): Promise<RequestPermissionResponse> {
    return new Promise((resolve) => {
      const key = params.toolCall.toolCallId;
      this.pendingPermissions.set(key, {
        request: this.normalizePermissionRequest(params),
        resolver: resolve,
      });
      this.syncPendingPermissionRequests();
    });
  }

  resolvePermission(requestId: string, optionId: string): void {
    const pendingRequest = this.pendingPermissions.get(requestId);
    if (!pendingRequest) {
      return;
    }

    this.pendingPermissions.delete(requestId);
    this.syncPendingPermissionRequests();
    pendingRequest.resolver({
      outcome: { outcome: 'selected', optionId },
      _meta: null,
    });
  }

  cancelPermission(requestId: string): void {
    const pendingRequest = this.pendingPermissions.get(requestId);
    if (!pendingRequest) {
      return;
    }

    this.pendingPermissions.delete(requestId);
    this.syncPendingPermissionRequests();
    pendingRequest.resolver({
      outcome: { outcome: 'cancelled' },
      _meta: null,
    });
  }

  getPermissions(): LocalPermissionRequest[] {
    return [...this.pendingPermissionRequests.value];
  }

  private normalizePermissionRequest(params: RequestPermissionRequest): LocalPermissionRequest {
    return {
      sessionId: params.sessionId,
      toolCall: {
        toolCallId: params.toolCall.toolCallId,
        title: params.toolCall.title ?? '',
        kind: params.toolCall.kind ?? 'other',
        status:
          (params.toolCall.status as
            | 'pending'
            | 'in_progress'
            | 'completed'
            | 'failed'
            | 'cancelled') ?? 'pending',
        locations: params.toolCall.locations?.map(({ path, line }) =>
          line == null ? { path } : { path, line }
        ),
        content: params.toolCall.content?.map((item) => ({ ...item })) ?? undefined,
        rawInput: normalizePermissionToolCallPayload(params.toolCall.rawInput),
        rawOutput: normalizePermissionToolCallPayload(params.toolCall.rawOutput),
        _meta: params.toolCall._meta ?? null,
      },
      options: params.options.map((option) => ({
        kind: option.kind,
        name: option.name,
        optionId: option.optionId,
        _meta: option._meta ?? null,
      })),
      _meta: params._meta ?? null,
    };
  }

  private syncPendingPermissionRequests(): void {
    this.pendingPermissionRequests.value = Array.from(this.pendingPermissions.values()).map(
      ({ request }) => request
    );
  }
}
