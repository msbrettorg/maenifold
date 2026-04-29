import { describe, expect, it } from 'vitest';
import { buildPermissionEquivalenceKey, canonicalPermissionTarget } from '../equivalence-key';
import type { PermissionRequest } from '../../types';

function request(overrides: Partial<PermissionRequest['toolCall']> = {}): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title: 'Read file',
      kind: 'read',
      status: 'pending',
      rawInput: { path: '/repo/file.txt' },
      ...overrides,
    },
    options: [],
  };
}

describe('buildPermissionEquivalenceKey', () => {
  it('separates otherwise identical requests by agent', () => {
    const left = buildPermissionEquivalenceKey(request(), 'agent-a', 'agent');
    const right = buildPermissionEquivalenceKey(request(), 'agent-b', 'agent');

    expect(left).not.toBe(right);
  });

  it('separates otherwise identical requests by scope window', () => {
    const left = buildPermissionEquivalenceKey(request(), 'agent-a', 'session:one');
    const right = buildPermissionEquivalenceKey(request(), 'agent-a', 'agent');

    expect(left).not.toBe(right);
  });

  it('uses the canonical absolute path for file-system requests', () => {
    const left = buildPermissionEquivalenceKey(request({ rawInput: { path: '/repo/a.txt' } }), 'agent-a', 'agent');
    const right = buildPermissionEquivalenceKey(request({ rawInput: { path: '/repo/b.txt' } }), 'agent-a', 'agent');

    expect(left).not.toBe(right);
    expect(left).toContain('/repo/a.txt');
  });

  it('uses command plus cwd for execute requests', () => {
    const left = buildPermissionEquivalenceKey(
      request({ kind: 'execute', rawInput: { command: 'npm test', cwd: '/repo' } }),
      'agent-a',
      'session:one'
    );
    const right = buildPermissionEquivalenceKey(
      request({ kind: 'execute', rawInput: { command: 'npm test', cwd: '/other' } }),
      'agent-a',
      'session:one'
    );

    expect(left).not.toBe(right);
    expect(left).toContain('npm test');
    expect(left).toContain('/repo');
  });

  it('uses only URL host for fetch requests', () => {
    const left = buildPermissionEquivalenceKey(
      request({ kind: 'fetch', rawInput: { url: 'https://example.com/a?x=1' } }),
      'agent-a',
      'agent'
    );
    const right = buildPermissionEquivalenceKey(
      request({ kind: 'fetch', rawInput: { url: 'https://example.com/b?x=2' } }),
      'agent-a',
      'agent'
    );

    expect(left).toBe(right);
    expect(left).toContain('example.com');
    expect(left).not.toContain('/a');
  });

  it('omits target for think requests', () => {
    const left = buildPermissionEquivalenceKey(
      request({ kind: 'think', rawInput: { prompt: 'first' } }),
      'agent-a',
      'agent'
    );
    const right = buildPermissionEquivalenceKey(
      request({ kind: 'think', rawInput: { prompt: 'second' } }),
      'agent-a',
      'agent'
    );

    expect(left).toBe(right);
  });

  it('separates requests by risk class metadata', () => {
    const low = buildPermissionEquivalenceKey(
      request({ _meta: { riskClass: 'low' } }),
      'agent-a',
      'agent'
    );
    const high = buildPermissionEquivalenceKey(
      request({ _meta: { riskClass: 'high' } }),
      'agent-a',
      'agent'
    );

    expect(low).not.toBe(high);
  });
});

describe('canonicalPermissionTarget', () => {
  it('does not use changing titles as targets for other requests', () => {
    const left = canonicalPermissionTarget(request({
      title: 'Discovering MCP tools',
      kind: 'other',
      rawInput: {},
      _meta: { riskClass: 'low' },
    }));
    const right = canonicalPermissionTarget(request({
      title: 'Finding MCP get-time tool',
      kind: 'other',
      rawInput: {},
      _meta: { riskClass: 'low' },
    }));

    expect(left).toBe(right);
  });

  it('keeps file targets path-based instead of title-based', () => {
    expect(canonicalPermissionTarget(request({
      title: 'Edit foo',
      kind: 'edit',
      rawInput: { path: '/foo' },
    }))).toBe('/foo');
  });

  it('keeps execute targets command plus cwd', () => {
    expect(canonicalPermissionTarget(request({
      title: 'List directory',
      kind: 'execute',
      rawInput: { command: 'ls', cwd: '/' },
    }))).toBe('ls\u0000/');
  });

  it('keeps explicit other targets', () => {
    expect(canonicalPermissionTarget(request({
      kind: 'other',
      rawInput: { target: 'specific-thing' },
    }))).toBe('specific-thing');
  });
});
