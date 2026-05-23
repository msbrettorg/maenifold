/**
 * ACP permission handling. Plan mode fails closed: only normalized tool kinds in
 * PLAN_MODE_SAFE_TOOL_KINDS are auto-eligible; every other kind auto-rejects
 * because unknown or write-like classifications may have side effects.
 */
import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { ref, type Ref } from 'vue';
import { normalizePermissionToolCallPayload } from './acp-meta';
import {
  buildPlanModeRejectionMessage,
  promptForPermission,
  syncPendingPermissionRequests,
  type InlinePermissionMessageSink as PromptInlinePermissionMessageSink,
  type PendingPermissionEntry,
} from './acp-permission-prompts';
import {
  decisionFromResponse,
  mostRestrictiveResponse,
  selectedResponse,
  tryModeSupersede,
} from './acp-permission-policy';
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
import type { PermissionEvaluateRequestResult } from './permission-ledger/types';

export type InlinePermissionMessageSink = PromptInlinePermissionMessageSink;

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
    const modeDecision = tryModeSupersede(request, this.context.currentModeId());
    if (modeDecision) {
      const kind = normalizePermissionToolKind(request.toolCall.kind);
      const toolName = request.toolCall.title || request.toolCall.kind || 'unknown';
      this.context.onInlineMessage?.({
        toolCallId: request.toolCall.toolCallId,
        message: buildPlanModeRejectionMessage(toolName, kind),
        severity: 'warning',
      });
      markAutoPermissionStatus(request.toolCall.toolCallId, modeDecision);
      try {
        return Promise.resolve(
          selectedResponse(request, modeDecision, (message) => {
            return new PermissionAgentNonCompliantError(message);
          })
        );
      } catch (error) {
        if (error instanceof PermissionAgentNonCompliantError) {
          pushPermissionSafetyNotification(
            `Agent failed to provide ${modeDecision} option for plan-mode protected tool call. Refusing for safety.`
          );
          return Promise.resolve(mostRestrictiveResponse(request));
        }
        throw error;
      }
    }

    const lookupContext = {
      agent: this.context.agent,
      sessionId: request.sessionId,
    };
    const ledgerDecision = this.permissionLedger.evaluateRequest
      ? this.permissionLedger.evaluateRequest(request, lookupContext)
      : this.permissionLedger.tryAutoDecide?.(request, lookupContext) ?? null;
    if (ledgerDecision instanceof Promise) {
      return ledgerDecision.then((decision) => this.resolveEvaluationOrPrompt(params, request, decision));
    }

    return this.resolveEvaluationOrPrompt(params, request, ledgerDecision);
  }

  private resolveEvaluationOrPrompt(
    params: RequestPermissionRequest,
    request: LocalPermissionRequest,
    ledgerDecision: PermissionEvaluateRequestResult | 'allow' | 'reject' | null
  ): Promise<RequestPermissionResponse> {
    if (typeof ledgerDecision === 'object' && ledgerDecision !== null) {
      if (ledgerDecision.autoDecided) {
        const decision = decisionFromResponse(request, ledgerDecision.response);
        if (decision) markAutoPermissionStatus(request.toolCall.toolCallId, decision);
        return Promise.resolve(ledgerDecision.response);
      }
      return promptForPermission(
        params,
        {
          ...request,
          toolCall: ledgerDecision.prompt.toolCall,
          options: ledgerDecision.prompt.options,
          decisionToken: ledgerDecision.token,
        },
        this.pendingPermissions,
        this.pendingPermissionRequests
      );
    }

    if (ledgerDecision) {
      markAutoPermissionStatus(request.toolCall.toolCallId, ledgerDecision);
      const event = this.permissionLedger.getLastAutoDecision?.();
      if (event) pushPermissionGhostNotification(event);
      try {
        return Promise.resolve(
          selectedResponse(request, ledgerDecision, (message) => {
            return new PermissionAgentNonCompliantError(message);
          })
        );
      } catch (error) {
        if (error instanceof PermissionAgentNonCompliantError) {
          pushPermissionSafetyNotification(
            `Agent failed to provide ${ledgerDecision} option for persisted permission decision. Showing the permission dialog instead.`
          );
          return promptForPermission(
            params,
            request,
            this.pendingPermissions,
            this.pendingPermissionRequests
          );
        }
        throw error;
      }
    }

    return promptForPermission(params, request, this.pendingPermissions, this.pendingPermissionRequests);
  }

  async resolvePermission(requestId: string, optionId: string, scope?: PermissionScope): Promise<void> {
    const pendingRequest = this.pendingPermissions.get(requestId);
    if (!pendingRequest) {
      return;
    }

    try {
      await this.recordPersistentDecision(pendingRequest.request, optionId, scope);
    } catch (error) {
      console.error('[T-244.102] Permission persistence failed:', error);
    } finally {
      this.pendingPermissions.delete(requestId);
      syncPendingPermissionRequests(this.pendingPermissions, this.pendingPermissionRequests);
      pendingRequest.resolver({
        outcome: { outcome: 'selected', optionId },
        _meta: null,
      });
    }
  }

  cancelPermission(requestId: string): void {
    const pendingRequest = this.pendingPermissions.get(requestId);
    if (!pendingRequest) {
      return;
    }

    this.pendingPermissions.delete(requestId);
    syncPendingPermissionRequests(this.pendingPermissions, this.pendingPermissionRequests);
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
    try {
      await this.permissionLedger.recordDecision(request, {
        agent: this.context.agent,
        scope,
        decision,
      });
    } catch (error) {
      console.error('Failed to persist permission decision via fallback ledger path:', error);
    }
  }
}
