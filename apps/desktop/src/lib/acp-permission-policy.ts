import type { RequestPermissionResponse } from '@agentclientprotocol/sdk';
import type { PermissionRequest as LocalPermissionRequest } from './types';
import { normalizePermissionToolKind } from './permission-ledger/equivalence-key';

const PLAN_MODE_SAFE_TOOL_KINDS = new Set(['read', 'fetch', 'search', 'think']);

export function tryModeSupersede(
  request: LocalPermissionRequest,
  modeId: string | null
): 'allow' | 'reject' | null {
  const kind = normalizePermissionToolKind(request.toolCall.kind);
  if (modeId !== 'plan' || PLAN_MODE_SAFE_TOOL_KINDS.has(kind)) {
    return null;
  }

  return 'reject';
}

export function decisionFromResponse(
  request: LocalPermissionRequest,
  response: RequestPermissionResponse
): 'allow' | 'reject' | null {
  const outcome = response.outcome;
  if (outcome.outcome !== 'selected') {
    return null;
  }
  const option = request.options.find((candidate) => candidate.optionId === outcome.optionId);
  if (option?.kind.startsWith('allow')) return 'allow';
  if (option?.kind.startsWith('reject')) return 'reject';
  return null;
}

export function selectedResponse(
  request: LocalPermissionRequest,
  decision: 'allow' | 'reject',
  createError: (message: string) => Error
): RequestPermissionResponse {
  const option = request.options.find(
    (candidate) => candidate.kind === `${decision}_once` || candidate.kind === `${decision}_always`
  );
  if (!option) {
    throw createError(
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

export function mostRestrictiveResponse(
  request: LocalPermissionRequest
): RequestPermissionResponse {
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
