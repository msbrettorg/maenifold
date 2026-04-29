import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import { describe, expect, it, vi } from 'vitest';
import { PermissionHandler } from '../acp-permissions';
import type { PermissionLedger } from '../permission-ledger/ledger';

function writePermissionParams(toolCallId = 'tool-write-1'): RequestPermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId,
      title: 'Write text file',
      kind: 'fs/write_text_file',
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

describe('PermissionHandler plan-mode fs/write visibility', () => {
  it('pushes a chat-visible inline message when plan mode blocks a write', async () => {
    const inlineMessageSink = vi.fn();
    const handler = new PermissionHandler(new StaticLedger(null), {
      agent: 'agent-a',
      currentModeId: () => 'plan',
      onInlineMessage: inlineMessageSink,
    } as never);

    const response = await handler.requestPermission(writePermissionParams());

    expect(response.outcome).toEqual({
      outcome: 'selected',
      optionId: 'tool-write-1-reject-once',
    });
    expect(inlineMessageSink).toHaveBeenCalledWith({
      toolCallId: 'tool-write-1',
      message: 'Write blocked by plan mode',
      severity: 'warning',
    });
  });

  it('stops auto-rejecting after the mode switches away from plan', async () => {
    let mode = 'plan';
    const inlineMessageSink = vi.fn();
    const handler = new PermissionHandler(new StaticLedger('allow'), {
      agent: 'agent-a',
      currentModeId: () => mode,
      onInlineMessage: inlineMessageSink,
    } as never);

    const rejected = await handler.requestPermission(writePermissionParams('tool-write-plan'));
    mode = 'architect';
    const allowed = await handler.requestPermission(writePermissionParams('tool-write-normal'));

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
