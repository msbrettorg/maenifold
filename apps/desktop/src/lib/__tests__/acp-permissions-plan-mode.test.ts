import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import { describe, expect, it, vi } from 'vitest';
import { PermissionHandler } from '../acp-permissions';
import type { PermissionLedger } from '../permission-ledger/ledger';

function permissionParams(
  toolCallId: string,
  kind: string,
  title = `${kind} tool`
): RequestPermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId,
      title,
      kind,
      status: 'pending',
      rawInput: {
        path: '/tmp/foo',
        content: 'hello',
      },
    },
    options: [
      { optionId: `${toolCallId}-allow-once`, kind: 'allow_once', name: 'Allow once' },
      { optionId: `${toolCallId}-reject-once`, kind: 'reject_once', name: 'Reject once' },
    ],
  } as unknown as RequestPermissionRequest;
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

describe('PermissionHandler plan-mode safe-kind whitelist', () => {
  it('does not auto-reject read tools in plan mode because read is safe', async () => {
    const inlineMessageSink = vi.fn();
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      agent: 'agent-a',
      currentModeId: () => 'plan',
      onInlineMessage: inlineMessageSink,
    } as never);

    const response = await handler.requestPermission(permissionParams('tool-read', 'read'));

    expect(response.outcome).toEqual({
      outcome: 'selected',
      optionId: 'tool-read-allow-once',
    });
    expect(inlineMessageSink).not.toHaveBeenCalled();
  });

  it.each(['query', 'list', 'view', 'inspect', 'describe', 'get'])(
    'does not auto-reject read-only alias kind %s in plan mode',
    async (kind) => {
      const inlineMessageSink = vi.fn();
      const handler = new PermissionHandler(new StaticLedger('allow'), {
        agent: 'agent-a',
        currentModeId: () => 'plan',
        onInlineMessage: inlineMessageSink,
      } as never);

      const response = await handler.requestPermission(permissionParams(`tool-${kind}`, kind));

      expect(response.outcome).toEqual({
        outcome: 'selected',
        optionId: `tool-${kind}-allow-once`,
      });
      expect(inlineMessageSink).not.toHaveBeenCalled();
    }
  );

  it('pushes a chat-visible inline message when plan mode blocks an edit', async () => {
    const inlineMessageSink = vi.fn();
    const handler = new PermissionHandler(new StaticLedger(null), {
      agent: 'agent-a',
      currentModeId: () => 'plan',
      onInlineMessage: inlineMessageSink,
    } as never);

    const response = await handler.requestPermission(
      permissionParams('tool-write-1', 'fs/write_text_file', 'Write text file')
    );

    expect(response.outcome).toEqual({
      outcome: 'selected',
      optionId: 'tool-write-1-reject-once',
    });
    expect(inlineMessageSink).toHaveBeenCalledWith({
      toolCallId: 'tool-write-1',
      message:
        "Plan mode auto-rejected tool 'Write text file' (kind 'edit' is not in the safe-kind whitelist).",
      severity: 'warning',
    });
  });

  it('auto-rejects unknown tool kinds in plan mode instead of failing open', async () => {
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      currentModeId: () => 'plan',
    } as never);

    const response = await handler.requestPermission(permissionParams('tool-unknown', 'unknown-kind'));

    expect(response.outcome).toEqual({
      outcome: 'selected',
      optionId: 'tool-unknown-reject-once',
    });
  });

  it.each(['write', 'modify', 'patch', 'apply'])(
    'auto-rejects custom write-like kind %s in plan mode',
    async (kind) => {
      const handler = new PermissionHandler(new StaticLedger('allow'), {
        currentModeId: () => 'plan',
      } as never);

      const response = await handler.requestPermission(permissionParams(`tool-${kind}`, kind));

      expect(response.outcome).toEqual({
        outcome: 'selected',
        optionId: `tool-${kind}-reject-once`,
      });
    }
  );

  it.each(['edit', 'unknown-kind', 'write', 'modify', 'patch', 'apply'])(
    'does not auto-reject kind %s when plan mode is disabled',
    async (kind) => {
      const inlineMessageSink = vi.fn();
      const handler = new PermissionHandler(new StaticLedger('allow'), {
        currentModeId: () => 'default',
        onInlineMessage: inlineMessageSink,
      } as never);

      const response = await handler.requestPermission(permissionParams(`tool-normal-${kind}`, kind));

      expect(response.outcome).toEqual({
        outcome: 'selected',
        optionId: `tool-normal-${kind}-allow-once`,
      });
      expect(inlineMessageSink).not.toHaveBeenCalled();
    }
  );

  it('stops auto-rejecting after the mode switches away from plan', async () => {
    let mode = 'plan';
    const inlineMessageSink = vi.fn();
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      agent: 'agent-a',
      currentModeId: () => mode,
      onInlineMessage: inlineMessageSink,
    } as never);

    const rejected = await handler.requestPermission(permissionParams('tool-write-plan', 'edit'));
    mode = 'architect';
    const allowed = await handler.requestPermission(permissionParams('tool-write-normal', 'apply'));

    expect(rejected.outcome).toEqual({
      outcome: 'selected',
      optionId: 'tool-write-plan-reject-once',
    });
    expect(allowed.outcome).toEqual({
      outcome: 'selected',
      optionId: 'tool-write-normal-allow-once',
    });
    expect(inlineMessageSink).toHaveBeenCalledTimes(1);
  });
});
