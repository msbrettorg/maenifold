import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { ref, type Ref } from 'vue';
import { normalizePermissionToolCallPayload } from './acp-meta';
import type { PermissionRequest as LocalPermissionRequest } from './types';
import {
  markAutoPermissionStatus,
  pushPermissionGhostNotification,
} from './permission-ledger/audit-events';
import {
  getDefaultPermissionLedger,
  type PermissionLedger,
  type PermissionScope,
} from './permission-ledger/ledger';
import { normalizePermissionToolKind } from './permission-ledger/equivalence-key';

type PermissionResolver = (response: RequestPermissionResponse) => void;

type PendingPermissionEntry = {
  request: LocalPermissionRequest;
  resolver: PermissionResolver;
};

export class PermissionHandler {
  public pendingPermissionRequests: Ref<LocalPermissionRequest[]> = ref([]);
  private pendingPermissions = new Map<string, PendingPermissionEntry>();
  private readonly permissionLedger: PermissionLedger;
  private readonly context: {
    agent: string;
    currentModeId: () => string | null;
  };

  constructor(
    permissionLedger: PermissionLedger = getDefaultPermissionLedger(),
    context: { agent?: string; currentModeId?: () => string | null } = {}
  ) {
    this.permissionLedger = permissionLedger;
    this.context = {
      agent: context.agent ?? '',
      currentModeId: context.currentModeId ?? (() => null),
    };
  }

  setCurrentModeProvider(currentModeId: () => string | null): void {
    this.context.currentModeId = currentModeId;
  }

  requestPermission(params: RequestPermissionRequest): Promise<RequestPermissionResponse> {
    const request = this.normalizePermissionRequest(params);
    const modeDecision = this.tryModeSupersede(request);
    if (modeDecision) {
      markAutoPermissionStatus(request.toolCall.toolCallId, modeDecision);
      return Promise.resolve(this.selectedResponse(request, modeDecision));
    }

    const ledgerDecision = this.permissionLedger.tryAutoDecide(request, {
      agent: this.context.agent,
      sessionId: request.sessionId,
    });
    if (ledgerDecision instanceof Promise) {
      return ledgerDecision.then((decision) => this.resolveAutoOrPrompt(params, request, decision));
    }

    return this.resolveAutoOrPrompt(params, request, ledgerDecision);
  }

  private resolveAutoOrPrompt(
    params: RequestPermissionRequest,
    request: LocalPermissionRequest,
    ledgerDecision: 'allow' | 'reject' | null
  ): Promise<RequestPermissionResponse> {
    if (ledgerDecision) {
      markAutoPermissionStatus(request.toolCall.toolCallId, ledgerDecision);
      const event = this.permissionLedger.getLastAutoDecision?.();
      if (event) pushPermissionGhostNotification(event);
      return Promise.resolve(this.selectedResponse(request, ledgerDecision));
    }

    return new Promise((resolve) => {
      const key = params.toolCall.toolCallId;
      this.pendingPermissions.set(key, {
        request,
        resolver: resolve,
      });
      this.syncPendingPermissionRequests();
    });
  }

  resolvePermission(requestId: string, optionId: string, scope?: PermissionScope): void {
    const pendingRequest = this.pendingPermissions.get(requestId);
    if (!pendingRequest) {
      return;
    }

    this.recordPersistentDecision(pendingRequest.request, optionId, scope);
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

  private tryModeSupersede(request: LocalPermissionRequest): 'allow' | 'reject' | null {
    const modeId = this.context.currentModeId();
    if (modeId === 'plan' && normalizePermissionToolKind(request.toolCall.kind) === 'edit') {
      return 'reject';
    }
    return null;
  }

  private selectedResponse(
    request: LocalPermissionRequest,
    decision: 'allow' | 'reject'
  ): RequestPermissionResponse {
    const option = request.options.find((candidate) => candidate.kind === `${decision}_once`);
    return {
      outcome: {
        outcome: 'selected',
        optionId: option?.optionId ?? request.options[0]?.optionId ?? '',
      },
      _meta: null,
    };
  }

  private recordPersistentDecision(
    request: LocalPermissionRequest,
    optionId: string,
    scope: PermissionScope | undefined
  ): void {
    const option = request.options.find((candidate) => candidate.optionId === optionId);
    if (!option || !option.kind.endsWith('_always') || !scope) {
      return;
    }

    const decision = option.kind.startsWith('allow') ? 'allow' : 'reject';
    void this.permissionLedger.recordDecision(request, {
      agent: this.context.agent,
      scope,
      decision,
    });
  }
}
