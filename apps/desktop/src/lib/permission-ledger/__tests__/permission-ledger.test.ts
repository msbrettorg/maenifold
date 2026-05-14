import { rmSync } from 'node:fs';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { NodeSqlitePermissionLedger } from '../node-sqlite-ledger';
import type {
  PermissionAuditEvent,
  PermissionDecision,
  PermissionLookupContext,
  PermissionScope,
} from '../ledger';
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

function otherRequest(title: string): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title,
      kind: 'other',
      status: 'pending',
      rawInput: {},
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
      { optionId: 'reject-once', kind: 'reject_once', name: 'Reject once' },
      { optionId: 'reject-always', kind: 'reject_always', name: 'Reject always' },
    ],
  };
}

const migrationTestDb = join(process.cwd(), '.permission-ledger-v1-migration-test.sqlite');

type ResolveAuditInput = {
  request: PermissionRequest | null;
  context: PermissionLookupContext;
  token: string;
  offeredOptionIds: string[] | null;
  selectedOptionId: string;
  decision: PermissionDecision | null;
  permissionId: number | null;
  rejectionReason: 'token_invalid' | 'token_expired' | 'option_mismatch' | 'scope_invalid' | null;
};

type ResolveAuditLedger = NodeSqlitePermissionLedger & {
  recordResolveAttempt(input: ResolveAuditInput): void;
};

function resolveAuditLedger(): ResolveAuditLedger {
  return new NodeSqlitePermissionLedger(':memory:') as ResolveAuditLedger;
}

function offeredOptionIds(input: PermissionRequest): string[] {
  return input.options.map((option) => option.optionId);
}

function recordResolveAttempt(
  ledger: ResolveAuditLedger,
  partial: Partial<ResolveAuditInput> = {}
): void {
  const input = request();
  ledger.recordResolveAttempt({
    request: input.request,
    context: { agent: input.agent, sessionId: 'session-1' },
    token: 'token-1',
    offeredOptionIds: offeredOptionIds(input.request),
    selectedOptionId: 'allow-once',
    decision: 'allow',
    permissionId: null,
    rejectionReason: null,
    ...partial,
  });
}

function latestAuditEvent(ledger: NodeSqlitePermissionLedger): PermissionAuditEvent {
  const [event] = ledger.listAuditEvents();
  if (!event) throw new Error('expected audit event');
  return event;
}

describe('NodeSqlitePermissionLedger', () => {
  beforeEach(() => {
    rmSync(migrationTestDb, { force: true });
  });

  afterEach(() => {
    rmSync(migrationTestDb, { force: true });
  });

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

  it('auto-allows other requests with different titles for the same agent', () => {
    const ledger = new NodeSqlitePermissionLedger(':memory:');

    ledger.recordDecision(otherRequest('A'), {
      agent: 'agent-a',
      scope: 'agent',
      decision: 'allow',
    });

    expect(ledger.tryAutoDecide(otherRequest('B'), { agent: 'agent-a', sessionId: 'session-1' })).toBe('allow');
  });

  it('records v3 audit fields when tryAutoDecide returns a stored decision', () => {
    const ledger = new NodeSqlitePermissionLedger(':memory:');
    const input = request();
    ledger.recordDecision(input.request, {
      agent: input.agent,
      scope: 'agent',
      decision: 'allow',
    });

    expect(ledger.tryAutoDecide(input.request, { agent: input.agent, sessionId: 'session-1' })).toBe('allow');

    const [event] = ledger.listAuditEvents();
    expect(event).toMatchObject({
      source: 'main_auto_decide',
      tokenId: null,
      offeredOptionIds: ['allow-once', 'allow-always', 'reject-once', 'reject-always'],
      selectedOptionId: 'allow-once',
      rejectionReason: null,
    });
  });

  it('records renderer_user_choice audit fields for successful allow_once resolve attempts', () => {
    const ledger = resolveAuditLedger();

    recordResolveAttempt(ledger);

    const event = latestAuditEvent(ledger);
    expect(event).toMatchObject({
      source: 'renderer_user_choice',
      tokenId: 'token-1',
      offeredOptionIds: ['allow-once', 'allow-always', 'reject-once', 'reject-always'],
      selectedOptionId: 'allow-once',
      decision: 'allow',
      rejectionReason: null,
      permissionId: null,
    });
  });

  it('records renderer_user_choice audit fields and permission id for successful allow_always resolve attempts', () => {
    const ledger = resolveAuditLedger();
    const input = request();
    const saved = ledger.recordDecision(input.request, {
      agent: input.agent,
      scope: 'agent' satisfies PermissionScope,
      decision: 'allow',
    });
    recordResolveAttempt(ledger, {
      request: input.request,
      token: 'token-always',
      offeredOptionIds: offeredOptionIds(input.request),
      selectedOptionId: 'allow-always',
      decision: 'allow',
      permissionId: saved.id,
    });

    const event = latestAuditEvent(ledger);
    expect(event).toMatchObject({
      source: 'renderer_user_choice',
      tokenId: 'token-always',
      offeredOptionIds: ['allow-once', 'allow-always', 'reject-once', 'reject-always'],
      selectedOptionId: 'allow-always',
      decision: 'allow',
      rejectionReason: null,
    });
    expect(event.permissionId).toEqual(expect.any(Number));
  });

  it('records token_invalid audit fields for fabricated token resolve attempts', () => {
    const ledger = resolveAuditLedger();
    recordResolveAttempt(ledger, {
      request: null,
      context: { agent: 'unknown', sessionId: 'unknown' },
      token: 'fabricated-token',
      offeredOptionIds: null,
      selectedOptionId: 'allow-once',
      decision: null,
      permissionId: null,
      rejectionReason: 'token_invalid',
    });

    const event = latestAuditEvent(ledger);
    expect(event).toMatchObject({
      source: 'renderer_user_choice',
      tokenId: 'fabricated-token',
      offeredOptionIds: null,
      selectedOptionId: 'allow-once',
      decision: null,
      rejectionReason: 'token_invalid',
    });
  });

  it('records token_expired audit fields for expired token resolve attempts', () => {
    const ledger = resolveAuditLedger();
    recordResolveAttempt(ledger, {
      request: null,
      context: { agent: 'unknown', sessionId: 'unknown' },
      token: 'expired-token',
      offeredOptionIds: null,
      selectedOptionId: 'allow-once',
      decision: null,
      permissionId: null,
      rejectionReason: 'token_expired',
    });

    const event = latestAuditEvent(ledger);
    expect(event).toMatchObject({
      source: 'renderer_user_choice',
      tokenId: 'expired-token',
      offeredOptionIds: null,
      selectedOptionId: 'allow-once',
      decision: null,
      rejectionReason: 'token_expired',
    });
  });

  it('records option_mismatch audit fields for unoffered optionId resolve attempts', () => {
    const ledger = resolveAuditLedger();
    recordResolveAttempt(ledger, {
      request: null,
      context: { agent: 'unknown', sessionId: 'unknown' },
      token: 'token-with-bad-option',
      offeredOptionIds: null,
      selectedOptionId: 'definitely-not-an-option',
      decision: null,
      permissionId: null,
      rejectionReason: 'option_mismatch',
    });

    const event = latestAuditEvent(ledger);
    expect(event).toMatchObject({
      source: 'renderer_user_choice',
      tokenId: 'token-with-bad-option',
      offeredOptionIds: null,
      selectedOptionId: 'definitely-not-an-option',
      decision: null,
      rejectionReason: 'option_mismatch',
    });
  });

  it('records scope_invalid audit fields for _always resolve attempts without scope', () => {
    const ledger = resolveAuditLedger();
    recordResolveAttempt(ledger, {
      request: null,
      context: { agent: 'unknown', sessionId: 'unknown' },
      token: 'token-without-scope',
      offeredOptionIds: null,
      selectedOptionId: 'allow-always',
      decision: null,
      permissionId: null,
      rejectionReason: 'scope_invalid',
    });

    const event = latestAuditEvent(ledger);
    expect(event).toMatchObject({
      source: 'renderer_user_choice',
      tokenId: 'token-without-scope',
      offeredOptionIds: null,
      selectedOptionId: 'allow-always',
      decision: null,
      rejectionReason: 'scope_invalid',
    });
  });

  it('migrates v1 permission_audit rows and allows new rejection rows with null decisions', () => {
    const db = new DatabaseSync(migrationTestDb);
    db.exec(`
      CREATE TABLE permission_audit (
        id INTEGER PRIMARY KEY,
        permission_id INTEGER,
        agent TEXT NOT NULL,
        session_id TEXT NOT NULL,
        tool_kind TEXT NOT NULL,
        canonical_target TEXT,
        decision TEXT NOT NULL,
        created_at TEXT NOT NULL
      );
      INSERT INTO permission_audit(permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at)
      VALUES (42, 'legacy-agent', 'legacy-session', 'read', '/legacy/file.txt', 'allow', '2026-05-14T00:00:00.000Z');
    `);
    db.close();

    const ledger = new NodeSqlitePermissionLedger(migrationTestDb);

    const [legacy] = ledger.listAuditEvents();
    expect(legacy).toMatchObject({
      permissionId: 42,
      agent: 'legacy-agent',
      source: null,
      tokenId: null,
      offeredOptionIds: null,
      selectedOptionId: null,
      rejectionReason: null,
    });

    const migratedDb = new DatabaseSync(migrationTestDb);
    expect(() =>
      migratedDb.prepare(`
        INSERT INTO permission_audit(permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at, source, token_id, selected_option_id, rejection_reason)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).run(
        null,
        'agent-a',
        'session-1',
        'read',
        '/repo/file.txt',
        null,
        '2026-05-14T00:01:00.000Z',
        'renderer_user_choice',
        'fabricated-token',
        'allow-once',
        'token_invalid'
      )
    ).not.toThrow();
    migratedDb.close();
  });
});
