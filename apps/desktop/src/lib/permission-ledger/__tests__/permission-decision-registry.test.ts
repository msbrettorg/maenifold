import { describe, expect, it } from 'vitest';
import { createPermissionDecisionRegistry } from '../decision-registry';
import type { PermissionRequest } from '../../types';

function request(): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title: 'Read file',
      kind: 'read',
      status: 'pending',
      rawInput: { path: '/repo/file.txt' },
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
    ],
  };
}

describe('main-owned permission decision registry', () => {
  it('rejects invalid resolve-decision tokens without recording a decision', () => {
    const recorded: unknown[] = [];
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: (savedRequest, context) => {
        recorded.push({ savedRequest, context });
      },
    });
    registry.register(request(), { agent: 'agent-a', sessionId: 'session-1' });

    expect(() =>
      registry.resolve({ token: 'not-token-1', optionId: 'allow-always', scope: 'agent' })
    ).toThrow('invalid token');
    expect(recorded).toHaveLength(0);
  });
});
