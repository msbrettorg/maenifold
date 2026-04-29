import type {
  PermissionLookupContext,
  PermissionRecordContext,
  PermissionScope,
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

export type PermissionDecisionRegistry = {
  register(request: PermissionRequest, context: PermissionLookupContext): string;
  resolve(payload: ResolveDecisionPayload): void;
};

export function createPermissionDecisionRegistry(deps: {
  now?: () => number;
  tokenFactory: () => string;
  recordDecision: (request: PermissionRequest, context: PermissionRecordContext) => void;
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
      if (!entry || entry.resolved || entry.expiresAt < now()) {
        throw new Error('invalid token');
      }
      entry.resolved = true;
      pending.delete(payload.token);

      const option = entry.request.options.find((candidate) => candidate.optionId === payload.optionId);
      if (!option) throw new Error('invalid optionId');
      if (!option.kind.endsWith('_always')) return;
      if (!payload.scope) throw new Error('invalid scope');

      deps.recordDecision(entry.request, {
        agent: entry.context.agent,
        scope: payload.scope,
        decision: option.kind.startsWith('allow') ? 'allow' : 'reject',
      });
    },
  };
}
