import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createPermissionDecisionRegistry } from '../decision-registry';
import { NodeSqlitePermissionLedger } from '../node-sqlite-ledger';
import type { PermissionLookupContext } from '../ledger';
import type { PermissionRequest } from '../../types';

type ElectronApiMock = {
  permissionTryAutoDecide: (payload: {
    request: PermissionRequest;
    context: PermissionLookupContext;
  }) => Promise<{
    decision: 'allow' | 'reject' | null;
    event: unknown;
    token: string | null;
  }>;
  permissionResolveDecision: (payload: {
    token: string;
    optionId: string;
    scope?: 'session' | 'agent' | 'forever';
  }) => Promise<void>;
};

function permissionRequest(toolCallId: string): RequestPermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId,
      title: 'Read file',
      kind: 'read',
      status: 'pending',
      rawInput: { path: '/repo/file.txt' },
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
      { optionId: 'reject-once', kind: 'reject_once', name: 'Reject once' },
      { optionId: 'reject-always', kind: 'reject_always', name: 'Reject always' },
    ],
  } as RequestPermissionRequest;
}

async function waitForPendingPermission(handler: {
  getPermissions: () => PermissionRequest[];
}): Promise<PermissionRequest> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const [request] = handler.getPermissions();
    if (request) return request;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  throw new Error('permission dialog was not displayed');
}

describe('permission persistence integration', () => {
  afterEach(() => {
    Reflect.deleteProperty(window, 'electronAPI');
    vi.resetModules();
  });

  it('persists an always decision through the IPC token round-trip and auto-decides the next equivalent request', async () => {
    vi.resetModules();
    Reflect.deleteProperty(window, 'electronAPI');

    const ledgerModule = await import('../ledger');
    ledgerModule.getDefaultPermissionLedger();

    const database = new NodeSqlitePermissionLedger(':memory:');
    let tokenCounter = 0;
    const registry = createPermissionDecisionRegistry({
      tokenFactory: () => `token-${(tokenCounter += 1)}`,
      recordDecision: (request, context) => {
        database.recordDecision(request, context);
      },
    });
    const electronApi: ElectronApiMock = {
      async permissionTryAutoDecide({ request, context }) {
        const decision = database.tryAutoDecide(request, context);
        return {
          decision,
          event: database.getLastAutoDecision(),
          token: decision === null ? registry.register(request, context) : null,
        };
      },
      async permissionResolveDecision(payload) {
        registry.resolve(payload);
      },
    };
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: electronApi,
    });

    const { PermissionHandler } = await import('../../acp-permissions');
    const handler = new PermissionHandler(undefined, {
      agent: 'agent-a',
      currentModeId: () => null,
    });

    const firstResponse = handler.requestPermission(permissionRequest('tool-1'));
    const displayed = await waitForPendingPermission(handler);

    expect(displayed.decisionToken).toBe('token-1');

    await handler.resolvePermission('tool-1', 'allow-always', 'agent');

    await expect(firstResponse).resolves.toEqual({
      outcome: { outcome: 'selected', optionId: 'allow-always' },
      _meta: null,
    });
    expect(database.listDecisions()).toEqual([
      expect.objectContaining({
        agent: 'agent-a',
        scope: 'agent',
        toolKind: 'read',
        canonicalTarget: '/repo/file.txt',
        decision: 'allow',
      }),
    ]);

    const secondResponse = await handler.requestPermission(permissionRequest('tool-2'));

    expect(secondResponse).toEqual({
      outcome: { outcome: 'selected', optionId: 'allow-once' },
      _meta: null,
    });
    expect(handler.getPermissions()).toHaveLength(0);
    expect(database.listAuditEvents()).toEqual([
      expect.objectContaining({
        agent: 'agent-a',
        sessionId: 'session-1',
        toolKind: 'read',
        canonicalTarget: '/repo/file.txt',
        decision: 'allow',
      }),
    ]);
  });
});
