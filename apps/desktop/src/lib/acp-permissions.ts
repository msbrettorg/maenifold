import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { ref, type Ref } from 'vue';
import { normalizePermissionToolCallPayload } from './acp-meta';
import type { PermissionRequest as LocalPermissionRequest } from './types';
import {
  markAutoPermissionStatus,
  pushPermissionGhostNotification,
  pushPermissionSafetyNotification,
} from './permission-ledger/audit-events';
import {
  getDefaultPermissionLedger,
  type PermissionLedger,
  type PermissionScope,
} from './permission-ledger/ledger';
import { normalizePermissionToolKind } from './permission-ledger/equivalence-key';
export { setChatNotificationSink, type ChatNotificationSink } from './acp-chat-notifications';

type PermissionResolver = (response: RequestPermissionResponse) => void;
export type InlinePermissionMessageSink = (message: {
  toolCallId: string;
  message: string;
  severity: 'warning';
}) => void;

type PendingPermissionEntry = {
  request: LocalPermissionRequest;
  resolver: PermissionResolver;
};

export class PermissionAgentNonCompliantError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'PermissionAgentNonCompliantError';
  }
}

export class PermissionHandler {
  public pendingPermissionRequests: Ref<LocalPermissionRequest[]> = ref([]);
  private pendingPermissions = new Map<string, PendingPermissionEntry>();
  private readonly permissionLedger: PermissionLedger;
  private readonly context: {
    agent: string;
    currentModeId: () => string | null;
    onInlineMessage?: InlinePermissionMessageSink;
  };

  constructor(
    permissionLedger: PermissionLedger = getDefaultPermissionLedger(),
    context: {
      agent?: string;
      currentModeId?: () => string | null;
      onInlineMessage?: InlinePermissionMessageSink;
    } = {}
  ) {
    this.permissionLedger = permissionLedger;
    this.context = {
      agent: context.agent ?? '',
      currentModeId: context.currentModeId ?? (() => null),
      onInlineMessage: context.onInlineMessage,
    };
  }

  setCurrentModeProvider(currentModeId: () => string | null): void {
    this.context.currentModeId = currentModeId;
  }

  setInlineMessageSink(onInlineMessage: InlinePermissionMessageSink): void {
    this.context.onInlineMessage = onInlineMessage;
  }

  requestPermission(params: RequestPermissionRequest): Promise<RequestPermissionResponse> {
    const request = this.normalizePermissionRequest(params);
    const modeDecision = this.tryModeSupersede(request);
    if (modeDecision) {
      markAutoPermissionStatus(request.toolCall.toolCallId, modeDecision);
      try {
        return Promise.resolve(this.selectedResponse(request, modeDecision));
      } catch (error) {
        if (error instanceof PermissionAgentNonCompliantError) {
          pushPermissionSafetyNotification(
            `Agent failed to provide ${modeDecision} option for plan-mode protected tool call. Refusing for safety.`
          );
          return Promise.resolve(this.mostRestrictiveResponse(request));
        }
        throw error;
      }
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
      try {
        return Promise.resolve(this.selectedResponse(request, ledgerDecision));
      } catch (error) {
        if (error instanceof PermissionAgentNonCompliantError) {
          pushPermissionSafetyNotification(
            `Agent failed to provide ${ledgerDecision} option for persisted permission decision. Showing the permission dialog instead.`
          );
          return this.promptForPermission(params, request);
        }
        throw error;
      }
    }

    return this.promptForPermission(params, request);
  }

  async resolvePermission(requestId: string, optionId: string, scope?: PermissionScope): Promise<void> {
    const pendingRequest = this.pendingPermissions.get(requestId);
    if (!pendingRequest) {
      return;
    }

    await this.recordPersistentDecision(pendingRequest.request, optionId, scope);
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
      if (request.toolCall.kind === 'fs/write_text_file') {
        this.context.onInlineMessage?.({
          toolCallId: request.toolCall.toolCallId,
          message: 'Write blocked by plan mode',
          severity: 'warning',
        });
      }
      return 'reject';
    }
    return null;
  }

  private selectedResponse(
    request: LocalPermissionRequest,
    decision: 'allow' | 'reject'
  ): RequestPermissionResponse {
    const option = request.options.find(
      (candidate) => candidate.kind === `${decision}_once` || candidate.kind === `${decision}_always`
    );
    if (!option) {
      throw new PermissionAgentNonCompliantError(
        `Agent did not provide a ${decision}_once option for tool call ${request.toolCall.toolCallId}`
      );
    }
    return {
      outcome: {
        outcome: 'selected',
        optionId: option.optionId,
      },
      _meta: null,
    };
  }

  private mostRestrictiveResponse(request: LocalPermissionRequest): RequestPermissionResponse {
    const reject = request.options.find(
      (candidate) => candidate.kind === 'reject_once' || candidate.kind === 'reject_always'
    );
    if (reject) {
      return {
        outcome: { outcome: 'selected', optionId: reject.optionId },
        _meta: null,
      };
    }
    return {
      outcome: { outcome: 'cancelled' },
      _meta: null,
    };
  }

  private promptForPermission(
    params: RequestPermissionRequest,
    request: LocalPermissionRequest
  ): Promise<RequestPermissionResponse> {
    const token = this.permissionLedger.consumePendingDecisionToken?.(request.toolCall.toolCallId);
    const displayRequest = token ? { ...request, decisionToken: token } : request;
    return new Promise((resolve) => {
      const key = params.toolCall.toolCallId;
      this.pendingPermissions.set(key, {
        request: displayRequest,
        resolver: resolve,
      });
      this.syncPendingPermissionRequests();
    });
  }

  private async recordPersistentDecision(
    request: LocalPermissionRequest,
    optionId: string,
    scope: PermissionScope | undefined
  ): Promise<void> {
    const option = request.options.find((candidate) => candidate.optionId === optionId);
    if (!option) {
      return;
    }

    if (request.decisionToken && this.permissionLedger.resolveDecision) {
      await this.permissionLedger.resolveDecision({
        token: request.decisionToken,
        optionId,
        scope,
      });
      return;
    }

    if (!option.kind.endsWith('_always') || !scope) {
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
