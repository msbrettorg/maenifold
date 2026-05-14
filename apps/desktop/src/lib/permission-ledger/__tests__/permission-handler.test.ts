import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PermissionHandler } from '../../acp-permissions';
import type { PermissionLedger, PermissionLookupContext } from '../ledger';
import type { PermissionRequest } from '../../types';

type PermissionPromptPayload = {
  toolCall: PermissionRequest['toolCall'];
  options: PermissionRequest['options'];
  message?: string;
};

type PermissionEvaluateRequestResult =
  | { autoDecided: true; response: RequestPermissionResponse }
  | { autoDecided: false; token: string; prompt: PermissionPromptPayload };

type EvaluateElectronApi = {
  permissionEvaluateRequest: (payload: {
    request: PermissionRequest;
    context: PermissionLookupContext;
  }) => Promise<PermissionEvaluateRequestResult>;
  permissionResolveDecision: (payload: {
    token: string;
    optionId: string;
    scope?: 'session' | 'agent' | 'forever';
  }) => Promise<void>;
  permissionListDecisions: () => Promise<unknown[]>;
};

function params(path = '/repo/file.txt'): RequestPermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: `tool-${path}`,
      title: 'Read file',
      kind: 'read',
      status: 'pending',
      rawInput: { path },
    },
    options: [
      { optionId: `allow-once-${path}`, kind: 'allow_once', name: 'Allow once' },
      { optionId: `allow-always-${path}`, kind: 'allow_always', name: 'Allow always' },
      { optionId: `reject-once-${path}`, kind: 'reject_once', name: 'Reject once' },
      { optionId: `reject-always-${path}`, kind: 'reject_always', name: 'Reject always' },
    ],
  } as RequestPermissionRequest;
}

class StaticLedger implements PermissionLedger {
  constructor(private readonly decision: 'allow' | 'reject' | null) {}
  tryAutoDecide(): 'allow' | 'reject' | null { return this.decision; }
  recordDecision(): never { throw new Error('not needed'); }
  listDecisions(): never { throw new Error('not needed'); }
  listAuditEvents(): never { throw new Error('not needed'); }
  revokeDecision(): never { throw new Error('not needed'); }
  resetDecisions(): never { throw new Error('not needed'); }
}

async function settleResponse(
  response: Promise<RequestPermissionResponse>
): Promise<RequestPermissionResponse | 'pending'> {
  return Promise.race([
    response,
    new Promise<'pending'>((resolve) => setTimeout(() => resolve('pending'), 0)),
  ]);
}

describe('PermissionHandler persistence interceptor', () => {
  it('auto-selects the current allow_once option for stored allow_always decisions', async () => {
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      agent: 'agent-a',
      currentModeId: () => null,
    });

    const response = await handler.requestPermission(params('/repo/file.txt'));

    expect(response.outcome).toEqual({ outcome: 'selected', optionId: 'allow-once-/repo/file.txt' });
    expect(handler.pendingPermissionRequests.value).toHaveLength(0);
  });

  it('auto-selects the current reject_once option for stored reject_always decisions', async () => {
    const handler = new PermissionHandler(new StaticLedger('reject'), {
      agent: 'agent-a',
      currentModeId: () => null,
    });

    const response = await handler.requestPermission(params('/repo/file.txt'));

    expect(response.outcome).toEqual({ outcome: 'selected', optionId: 'reject-once-/repo/file.txt' });
    expect(handler.pendingPermissionRequests.value).toHaveLength(0);
  });

  it('still prompts when the ledger has no equivalent decision', () => {
    const handler = new PermissionHandler(new StaticLedger(null), {
      agent: 'agent-a',
      currentModeId: () => null,
    });

    void handler.requestPermission(params('/repo/file.txt'));

    expect(handler.pendingPermissionRequests.value).toHaveLength(1);
  });

  it('checks plan mode before the ledger and auto-denies edits', async () => {
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      agent: 'agent-a',
      currentModeId: () => 'plan',
    });
    const edit = params('/repo/file.txt') as RequestPermissionRequest;
    edit.toolCall.kind = 'edit';

    const response = await handler.requestPermission(edit);

    expect(response.outcome).toEqual({ outcome: 'selected', optionId: 'reject-once-/repo/file.txt' });
    expect(handler.pendingPermissionRequests.value).toHaveLength(0);
  });

  it('does not fall back to allow when plan-mode reject options are missing', async () => {
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      agent: 'agent-a',
      currentModeId: () => 'plan',
    });
    const edit = params('/repo/file.txt') as RequestPermissionRequest;
    edit.toolCall.kind = 'edit';
    edit.options = [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
    ];

    const response = await handler.requestPermission(edit);

    expect(response.outcome).not.toEqual({ outcome: 'selected', optionId: 'allow-once' });
    expect(handler.pendingPermissionRequests.value).toHaveLength(0);
  });

  it('records an always decision with the selected scope before resolving', () => {
    const records: Array<{ request: PermissionRequest; scope: string; decision: string }> = [];
    const ledger: PermissionLedger = {
      tryAutoDecide: () => null,
      recordDecision: (request, context) => {
        records.push({ request, scope: context.scope, decision: context.decision });
        return { id: 1 } as never;
      },
      listDecisions: () => [],
      listAuditEvents: () => [],
      revokeDecision: () => {},
      resetDecisions: () => {},
    };
    const handler = new PermissionHandler(ledger, { agent: 'agent-a', currentModeId: () => null });

    void handler.requestPermission(params('/repo/file.txt'));
    handler.resolvePermission('tool-/repo/file.txt', 'allow-always-/repo/file.txt', 'agent');

    expect(records).toEqual([
      expect.objectContaining({ scope: 'agent', decision: 'allow' }),
    ]);
  });
});

describe('PermissionHandler evaluate-request IPC contract', () => {
  afterEach(() => {
    Reflect.deleteProperty(window, 'electronAPI');
    vi.restoreAllMocks();
    vi.resetModules();
  });

  it('evaluateRequest auto-decide path returns response without prompting', async () => {
    vi.resetModules();
    const expectedResponse: RequestPermissionResponse = {
      outcome: { outcome: 'selected', optionId: 'allow-once-/repo/file.txt' },
      _meta: null,
    };
    const evaluateRequest = vi.fn<Parameters<EvaluateElectronApi['permissionEvaluateRequest']>, ReturnType<EvaluateElectronApi['permissionEvaluateRequest']>>()
      .mockResolvedValue({ autoDecided: true, response: expectedResponse });
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        permissionEvaluateRequest: evaluateRequest,
        permissionResolveDecision: vi.fn(),
        permissionListDecisions: vi.fn().mockResolvedValue([]),
      } satisfies EvaluateElectronApi,
    });
    const { PermissionHandler: RuntimePermissionHandler } = await import('../../acp-permissions');
    const handler = new RuntimePermissionHandler(undefined, {
      agent: 'agent-a',
      currentModeId: () => null,
    });

    const response = handler.requestPermission(params('/repo/file.txt'));

    await expect(settleResponse(response)).resolves.toEqual(expectedResponse);
    expect(handler.pendingPermissionRequests.value).toHaveLength(0);
    expect(evaluateRequest).toHaveBeenCalledTimes(1);
    expect(evaluateRequest).toHaveBeenCalledWith({
      request: expect.objectContaining({
        sessionId: 'session-1',
        toolCall: expect.objectContaining({ toolCallId: 'tool-/repo/file.txt' }),
      }),
      context: { agent: 'agent-a', sessionId: 'session-1' },
    });
  });

  it('evaluateRequest non-auto path returns token + prompt; renderer calls resolveDecision with user choice', async () => {
    vi.resetModules();
    const resolveDecision = vi.fn().mockResolvedValue(undefined);
    const evaluateRequest = vi.fn<Parameters<EvaluateElectronApi['permissionEvaluateRequest']>, ReturnType<EvaluateElectronApi['permissionEvaluateRequest']>>()
      .mockImplementation(async ({ request }) => ({
        autoDecided: false,
        token: '123e4567-e89b-12d3-a456-426614174000',
        prompt: {
          toolCall: request.toolCall,
          options: request.options,
        },
      }));
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        permissionEvaluateRequest: evaluateRequest,
        permissionResolveDecision: resolveDecision,
        permissionListDecisions: vi.fn().mockResolvedValue([]),
      } satisfies EvaluateElectronApi,
    });
    const { PermissionHandler: RuntimePermissionHandler } = await import('../../acp-permissions');
    const handler = new RuntimePermissionHandler(undefined, {
      agent: 'agent-a',
      currentModeId: () => null,
    });

    const response = handler.requestPermission(params('/repo/file.txt'));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(handler.pendingPermissionRequests.value).toEqual([
      expect.objectContaining({
        decisionToken: '123e4567-e89b-12d3-a456-426614174000',
        toolCall: expect.objectContaining({ toolCallId: 'tool-/repo/file.txt' }),
      }),
    ]);

    await handler.resolvePermission('tool-/repo/file.txt', 'allow-always-/repo/file.txt', 'agent');

    await expect(response).resolves.toEqual({
      outcome: { outcome: 'selected', optionId: 'allow-always-/repo/file.txt' },
      _meta: null,
    });
    expect(resolveDecision).toHaveBeenCalledWith({
      token: '123e4567-e89b-12d3-a456-426614174000',
      optionId: 'allow-always-/repo/file.txt',
      scope: 'agent',
    });
  });

  it('IpcPermissionLedger no longer has tryAutoDecide method', async () => {
    vi.resetModules();
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        permissionEvaluateRequest: vi.fn(),
        permissionResolveDecision: vi.fn(),
        permissionListDecisions: vi.fn().mockResolvedValue([]),
      } satisfies EvaluateElectronApi,
    });
    const { createDefaultPermissionLedger } = await import('../ledger');

    const ledger = createDefaultPermissionLedger();

    expect('tryAutoDecide' in ledger).toBe(false);
  });

  it('createDefaultPermissionLedger returns IpcPermissionLedger when permissionEvaluateRequest is exposed', async () => {
    vi.resetModules();
    const persisted = [{ id: 30420 }];
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        permissionEvaluateRequest: vi.fn(),
        permissionResolveDecision: vi.fn(),
        permissionListDecisions: vi.fn().mockResolvedValue(persisted),
      } satisfies EvaluateElectronApi,
    });
    const { createDefaultPermissionLedger } = await import('../ledger');

    const ledger = createDefaultPermissionLedger();

    await expect(Promise.resolve(ledger.listDecisions())).resolves.toBe(persisted);
  });
});
