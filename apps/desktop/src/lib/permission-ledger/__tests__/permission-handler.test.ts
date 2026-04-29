import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import { PermissionHandler } from '../../acp-permissions';
import type { PermissionLedger } from '../ledger';
import type { PermissionRequest } from '../../types';

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
