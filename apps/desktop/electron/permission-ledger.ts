import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { app, ipcMain } from 'electron';
import {
  PermissionResolveError,
  type PermissionDecision,
  type PermissionLookupContext,
  type PermissionScope,
} from '../src/lib/permission-ledger/ledger.js';
import { createPermissionDecisionRegistry } from '../src/lib/permission-ledger/decision-registry.js';
import { NodeSqlitePermissionLedger } from '../src/lib/permission-ledger/node-sqlite-ledger.js';
import type {
  PermissionEvaluateRequestResult,
  PermissionPromptPayload,
} from '../src/lib/permission-ledger/types.js';
import type { PermissionRequest } from '../src/lib/types.js';
import type { RequestPermissionResponse } from '@agentclientprotocol/sdk';

function decisionFromOptionKind(optionKind: string): PermissionDecision {
  return optionKind.startsWith('allow') ? 'allow' : 'reject';
}

function defaultDatabasePath(): string {
  return join(app.getPath('home'), '.maenifold', 'permissions.db');
}

let ledger: NodeSqlitePermissionLedger | null = null;

function getLedger(): NodeSqlitePermissionLedger {
  ledger ??= new NodeSqlitePermissionLedger(defaultDatabasePath());
  return ledger;
}

const RESET_CONFIRM_TTL_MS = 30 * 1000;

type ResolveDecisionPayload = {
  token: string;
  optionId: string;
  scope?: PermissionScope;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assertString(value: unknown, label: string): string {
  if (typeof value !== 'string') throw new Error(`invalid ${label}`);
  return value;
}

function assertOptionalScope(value: unknown): PermissionScope | undefined {
  if (value === undefined) return undefined;
  if (value !== 'session' && value !== 'agent' && value !== 'forever') {
    throw new Error('invalid scope');
  }
  return value;
}

function assertPermissionRequest(value: unknown): PermissionRequest {
  if (!isRecord(value)) throw new Error('invalid request');
  const sessionId = assertString(value.sessionId, 'request.sessionId');
  if (!isRecord(value.toolCall)) throw new Error('invalid request.toolCall');
  const toolCall = value.toolCall;
  assertString(toolCall.toolCallId, 'request.toolCall.toolCallId');
  assertString(toolCall.kind, 'request.toolCall.kind');
  if (!Array.isArray(value.options)) throw new Error('invalid request.options');
  for (const option of value.options) {
    if (!isRecord(option)) throw new Error('invalid request.options');
    assertString(option.optionId, 'request.options.optionId');
    assertString(option.kind, 'request.options.kind');
    assertString(option.name, 'request.options.name');
  }
  return { ...(value as PermissionRequest), sessionId };
}

function assertLookupContext(value: unknown): PermissionLookupContext {
  if (!isRecord(value)) throw new Error('invalid context');
  return {
    agent: assertString(value.agent, 'context.agent'),
    sessionId: assertString(value.sessionId, 'context.sessionId'),
  };
}

function assertEvaluateRequestPayload(payload: unknown): {
  request: PermissionRequest;
  context: PermissionLookupContext;
} {
  if (!isRecord(payload)) throw new Error('invalid payload');
  return {
    request: assertPermissionRequest(payload.request),
    context: assertLookupContext(payload.context),
  };
}

function selectedResponse(
  request: PermissionRequest,
  decision: PermissionDecision
): RequestPermissionResponse {
  const option = request.options.find(
    (candidate) => candidate.kind === `${decision}_once` || candidate.kind === `${decision}_always`
  );
  if (!option) {
    throw new Error(`missing ${decision} permission option`);
  }
  return {
    outcome: { outcome: 'selected', optionId: option.optionId },
    _meta: null,
  };
}

function permissionPromptPayload(request: PermissionRequest): PermissionPromptPayload {
  return {
    toolCall: request.toolCall,
    options: request.options,
  };
}

function assertResolveDecisionPayload(payload: unknown): ResolveDecisionPayload {
  if (!isRecord(payload)) throw new Error('invalid payload');
  return {
    token: assertString(payload.token, 'token'),
    optionId: assertString(payload.optionId, 'optionId'),
    scope: assertOptionalScope(payload.scope),
  };
}

function assertPositiveInteger(value: unknown, label: string): number {
  if (!Number.isInteger(value) || (value as number) <= 0) throw new Error(`invalid ${label}`);
  return value as number;
}

const decisionRegistry = createPermissionDecisionRegistry({
  tokenFactory: () => randomUUID(),
  recordDecision: (request, context) => {
    return getLedger().recordDecision(request, context);
  },
});

let resetConfirmToken: { token: string; expiresAt: number } | null = null;

export function registerPermissionLedgerHandlers(): void {
  ipcMain.handle('permission:evaluate-request', (_event, payload: unknown): PermissionEvaluateRequestResult => {
    const { request, context } = assertEvaluateRequestPayload(payload);
    const activeLedger = getLedger();
    const decision = activeLedger.tryAutoDecide(request, context);
    if (decision) {
      return { autoDecided: true, response: selectedResponse(request, decision) };
    }
    return {
      autoDecided: false,
      token: decisionRegistry.register(request, context),
      prompt: permissionPromptPayload(request),
    };
  });
  ipcMain.handle('permission:resolve-decision', (_event, payload: unknown) => {
    const input = assertResolveDecisionPayload(payload);
    try {
      const result = decisionRegistry.resolve(input);
      const decision = decisionFromOptionKind(result.selectedOption.kind);
      getLedger().recordResolveAttempt({
        request: result.request,
        context: result.context,
        token: input.token,
        offeredOptionIds: result.request.options.map((option) => option.optionId),
        selectedOptionId: input.optionId,
        decision,
        permissionId: result.persistedDecision?.id ?? null,
        rejectionReason: null,
      });
    } catch (err) {
      if (err instanceof PermissionResolveError) {
        getLedger().recordResolveAttempt({
          request: null,
          context: { agent: 'unknown', sessionId: 'unknown' },
          token: input.token,
          offeredOptionIds: null,
          selectedOptionId: input.optionId,
          decision: null,
          permissionId: null,
          rejectionReason: err.code,
        });
        throw new Error(JSON.stringify({ code: err.code, message: err.message }));
      }
      throw err;
    }
  });
  ipcMain.handle('permission:list-decisions', () => getLedger().listDecisions());
  ipcMain.handle('permission:list-audit-events', () => getLedger().listAuditEvents());
  ipcMain.handle('permission:revoke-decision', (_event, id: unknown) =>
    getLedger().revokeDecision(assertPositiveInteger(id, 'id'))
  );
  ipcMain.handle('permission:reset-confirm-prepare', () => {
    const token = randomUUID();
    resetConfirmToken = { token, expiresAt: Date.now() + RESET_CONFIRM_TTL_MS };
    return token;
  });
  ipcMain.handle('permission:reset-decisions', (_event, token: unknown) => {
    const input = assertString(token, 'token');
    if (!resetConfirmToken || resetConfirmToken.token !== input || resetConfirmToken.expiresAt < Date.now()) {
      throw new Error('invalid token');
    }
    resetConfirmToken = null;
    getLedger().resetDecisions();
  });
}
