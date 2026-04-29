import { describe, expect, it } from 'vitest';
import { NodeSqlitePermissionLedger } from '../node-sqlite-ledger';
import type { PermissionRequest } from '../../types';

function request(path = '/repo/file.txt', agent = 'agent-a'): { request: PermissionRequest; agent: string } {
  return {
    agent,
    request: {
      sessionId: 'session-1',
      toolCall: {
        toolCallId: 'tool-1',
        title: 'Read file',
        kind: 'read',
        status: 'pending',
        rawInput: { path },
      },
      options: [
        { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
        { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
        { optionId: 'reject-once', kind: 'reject_once', name: 'Reject once' },
        { optionId: 'reject-always', kind: 'reject_always', name: 'Reject always' },
      ],
    },
  };
}

describe('NodeSqlitePermissionLedger', () => {
  it('writes and reads persisted allow decisions', () => {
    const ledger = new NodeSqlitePermissionLedger(':memory:');
    const input = request();

    const saved = ledger.recordDecision(input.request, {
      agent: input.agent,
      scope: 'agent',
      decision: 'allow',
    });

    expect(saved.id).toBeGreaterThan(0);
    expect(ledger.tryAutoDecide(input.request, { agent: input.agent, sessionId: 'session-1' })).toBe('allow');
  });

  it('writes and reads persisted reject decisions', () => {
    const ledger = new NodeSqlitePermissionLedger(':memory:');
    const input = request();

    ledger.recordDecision(input.request, {
      agent: input.agent,
      scope: 'agent',
      decision: 'reject',
    });

    expect(ledger.tryAutoDecide(input.request, { agent: input.agent, sessionId: 'session-1' })).toBe('reject');
  });

  it('does not match different paths or different agents', () => {
    const ledger = new NodeSqlitePermissionLedger(':memory:');
    const input = request('/repo/file-a.txt', 'agent-a');

    ledger.recordDecision(input.request, {
      agent: input.agent,
      scope: 'agent',
      decision: 'allow',
    });

    expect(ledger.tryAutoDecide(request('/repo/file-b.txt', 'agent-a').request, { agent: 'agent-a', sessionId: 'session-1' })).toBeNull();
    expect(ledger.tryAutoDecide(request('/repo/file-a.txt', 'agent-b').request, { agent: 'agent-b', sessionId: 'session-1' })).toBeNull();
  });

  it('revokes individual decisions so equivalent requests prompt again', () => {
    const ledger = new NodeSqlitePermissionLedger(':memory:');
    const input = request();
    const saved = ledger.recordDecision(input.request, {
      agent: input.agent,
      scope: 'agent',
      decision: 'allow',
    });

    ledger.revokeDecision(saved.id);

    expect(ledger.tryAutoDecide(input.request, { agent: input.agent, sessionId: 'session-1' })).toBeNull();
  });
});
