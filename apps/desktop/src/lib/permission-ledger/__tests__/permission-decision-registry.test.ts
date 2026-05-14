import { describe, expect, it } from 'vitest';
import * as decisionRegistryModule from '../decision-registry';
import type { PermissionRequest } from '../../types';

const { createPermissionDecisionRegistry } = decisionRegistryModule;

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

function captureThrown(fn: () => void): unknown {
  try {
    fn();
  } catch (error) {
    return error;
  }
  throw new Error('expected function to throw');
}

function expectPermissionResolveError(error: unknown, code: string): void {
  const PermissionResolveError = (
    decisionRegistryModule as unknown as { PermissionResolveError?: new (...args: unknown[]) => Error }
  ).PermissionResolveError;
  expect(PermissionResolveError).toBeDefined();
  expect(error).toBeInstanceOf(PermissionResolveError);
  expect(error).toMatchObject({ code, message: code });
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

    const error = captureThrown(() =>
      registry.resolve({ token: 'not-token-1', optionId: 'allow-always', scope: 'agent' })
    );

    expect(error).toMatchObject({ message: 'token_invalid' });
    expectPermissionResolveError(error, 'token_invalid');
    expect(recorded).toHaveLength(0);
  });

  it('throws token_invalid PermissionResolveError for fabricated tokens', () => {
    const recorded: unknown[] = [];
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: (savedRequest, context) => {
        recorded.push({ savedRequest, context });
      },
    });

    const error = captureThrown(() =>
      registry.resolve({ token: 'fabricated-token', optionId: 'allow-always', scope: 'agent' })
    );

    expectPermissionResolveError(error, 'token_invalid');
    expect(recorded).toHaveLength(0);
  });

  it('collapses reused tokens to token_invalid PermissionResolveError', () => {
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: () => {},
    });
    const token = registry.register(request(), { agent: 'agent-a', sessionId: 'session-1' });
    registry.resolve({ token, optionId: 'allow-once' });

    const error = captureThrown(() => registry.resolve({ token, optionId: 'allow-once' }));

    expectPermissionResolveError(error, 'token_invalid');
  });

  it('throws token_expired PermissionResolveError after the ten minute TTL elapses', () => {
    let now = 1_000;
    const registry = createPermissionDecisionRegistry({
      now: () => now,
      tokenFactory: () => 'token-1',
      recordDecision: () => {},
    });
    const token = registry.register(request(), { agent: 'agent-a', sessionId: 'session-1' });
    now = 1_000 + 11 * 60 * 1_000;

    const error = captureThrown(() =>
      registry.resolve({ token, optionId: 'allow-always', scope: 'agent' })
    );

    expectPermissionResolveError(error, 'token_expired');
  });

  it('throws option_mismatch PermissionResolveError when optionId was not offered', () => {
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: () => {},
    });
    const token = registry.register(request(), { agent: 'agent-a', sessionId: 'session-1' });

    const error = captureThrown(() =>
      registry.resolve({ token, optionId: 'definitely-not-an-option', scope: 'agent' })
    );

    expectPermissionResolveError(error, 'option_mismatch');
  });

  it('throws scope_invalid PermissionResolveError for _always decisions without scope', () => {
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: () => {},
    });
    const token = registry.register(request(), { agent: 'agent-a', sessionId: 'session-1' });

    const error = captureThrown(() => registry.resolve({ token, optionId: 'allow-always' }));

    expectPermissionResolveError(error, 'scope_invalid');
  });

  it('resolves allow_once without recording a persisted decision', () => {
    const recorded: unknown[] = [];
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: (savedRequest, context) => {
        recorded.push({ savedRequest, context });
      },
    });
    const token = registry.register(request(), { agent: 'agent-a', sessionId: 'session-1' });

    expect(() => registry.resolve({ token, optionId: 'allow-once' })).not.toThrow();

    expect(recorded).toHaveLength(0);
  });

  it('resolves allow_always with scope and records the expected request context', () => {
    const recorded: unknown[] = [];
    const savedRequest = request();
    const registry = createPermissionDecisionRegistry({
      now: () => 1_000,
      tokenFactory: () => 'token-1',
      recordDecision: (persistedRequest, context) => {
        recorded.push({ request: persistedRequest, context });
      },
    });
    const token = registry.register(savedRequest, { agent: 'agent-a', sessionId: 'session-1' });

    expect(() =>
      registry.resolve({ token, optionId: 'allow-always', scope: 'agent' })
    ).not.toThrow();

    expect(recorded).toEqual([
      {
        request: savedRequest,
        context: { agent: 'agent-a', scope: 'agent', decision: 'allow' },
      },
    ]);
  });
});
