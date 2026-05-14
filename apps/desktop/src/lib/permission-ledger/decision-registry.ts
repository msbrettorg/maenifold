import type {
  PermissionLookupContext,
  PermissionRecordContext,
  PermissionScope,
  PersistedPermissionDecision,
} from './ledger';
import type { PermissionRequest } from '../types';

const DECISION_TOKEN_TTL_MS = 10 * 60 * 1000;

type PendingDecision = {
  request: PermissionRequest;
  context: PermissionLookupContext;
  expiresAt: number;
  resolved: boolean;
};

export type ResolveDecisionPayload = {
  token: string;
  optionId: string;
  scope?: PermissionScope;
};

export type PermissionRejectionReason =
  | 'token_invalid'
  | 'token_expired'
  | 'option_mismatch'
  | 'scope_invalid';

export class PermissionResolveError extends Error {
  readonly code: PermissionRejectionReason;

  constructor(code: PermissionRejectionReason) {
    super(code);
    this.name = 'PermissionResolveError';
    this.code = code;
  }
}

export type ResolvedPermissionDecision = {
  request: PermissionRequest;
  context: PermissionLookupContext;
  selectedOption: PermissionRequest['options'][number];
  persistedDecision: PersistedPermissionDecision | null;
};

export type PermissionDecisionRegistry = {
  register(request: PermissionRequest, context: PermissionLookupContext): string;
  resolve(payload: ResolveDecisionPayload): ResolvedPermissionDecision;
};

export function createPermissionDecisionRegistry(deps: {
  now?: () => number;
  tokenFactory: () => string;
  recordDecision: (
    request: PermissionRequest,
    context: PermissionRecordContext
  ) => PersistedPermissionDecision | void;
}): PermissionDecisionRegistry {
  const now = deps.now ?? (() => Date.now());
  const pending = new Map<string, PendingDecision>();

  return {
    register(request, context) {
      const token = deps.tokenFactory();
      pending.set(token, {
        request,
        context,
        expiresAt: now() + DECISION_TOKEN_TTL_MS,
        resolved: false,
      });
      return token;
    },
    resolve(payload) {
      const entry = pending.get(payload.token);
      if (!entry || entry.resolved) {
        throw new PermissionResolveError('token_invalid');
      }
      if (entry.expiresAt < now()) {
        throw new PermissionResolveError('token_expired');
      }
      entry.resolved = true;
      pending.delete(payload.token);

      const option = entry.request.options.find((candidate) => candidate.optionId === payload.optionId);
      if (!option) throw new PermissionResolveError('option_mismatch');
      if (!option.kind.endsWith('_always')) {
        return {
          request: entry.request,
          context: entry.context,
          selectedOption: option,
          persistedDecision: null,
        };
      }
      if (!payload.scope) throw new PermissionResolveError('scope_invalid');

      const persistedDecision = deps.recordDecision(entry.request, {
        agent: entry.context.agent,
        scope: payload.scope,
        decision: option.kind.startsWith('allow') ? 'allow' : 'reject',
      });
      return {
        request: entry.request,
        context: entry.context,
        selectedOption: option,
        persistedDecision: persistedDecision ?? null,
      };
    },
  };
}
