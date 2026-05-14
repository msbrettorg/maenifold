import { mkdirSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { dirname, join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { app, ipcMain } from 'electron';
import {
  canonicalPermissionTarget,
  normalizePermissionToolKind,
  permissionRiskClass,
} from '../src/lib/permission-ledger/equivalence-key.js';
import type {
  LastAutoPermissionDecision,
  PermissionAuditEvent,
  PermissionAuditSource,
  PermissionDecision,
  PermissionLookupContext,
  PermissionRecordContext,
  PermissionRejectionReason,
  PermissionScope,
  PersistedPermissionDecision,
} from '../src/lib/permission-ledger/ledger.js';
import {
  createPermissionDecisionRegistry,
  PermissionResolveError,
} from '../src/lib/permission-ledger/decision-registry.js';
import type {
  PermissionEvaluateRequestResult,
  PermissionPromptPayload,
} from '../src/lib/permission-ledger/types.js';
import type { PermissionRequest } from '../src/lib/types.js';
import type { RequestPermissionResponse } from '@agentclientprotocol/sdk';

type DbRow = {
  id: number;
  agent: string;
  scope: 'session' | 'agent' | 'forever';
  scope_session_id: string | null;
  tool_kind: string;
  canonical_target: string | null;
  risk_class: string | null;
  decision: PermissionDecision;
  created_at: string;
  last_used_at: string | null;
};

type AuditRow = {
  id: number;
  permissionId: number | null;
  agent: string;
  sessionId: string;
  toolKind: string;
  canonicalTarget: string | null;
  decision: PermissionDecision | null;
  createdAt: string;
  source: PermissionAuditSource | null;
  tokenId: string | null;
  offeredOptionIds: string | null;
  selectedOptionId: string | null;
  rejectionReason: PermissionRejectionReason | null;
};

type ResolveAuditInput = {
  request: PermissionRequest | null;
  context: PermissionLookupContext;
  token: string;
  offeredOptionIds: string[] | null;
  selectedOptionId: string;
  decision: PermissionDecision | null;
  permissionId: number | null;
  rejectionReason: PermissionRejectionReason | null;
};

function mapDecision(row: DbRow): PersistedPermissionDecision {
  return {
    id: row.id,
    agent: row.agent,
    scope: row.scope,
    scopeSessionId: row.scope_session_id,
    toolKind: row.tool_kind,
    canonicalTarget: row.canonical_target,
    riskClass: row.risk_class,
    decision: row.decision,
    createdAt: row.created_at,
    lastUsedAt: row.last_used_at,
  };
}

function mapAuditEvent(row: AuditRow): PermissionAuditEvent {
  return {
    ...row,
    offeredOptionIds: parseOfferedOptionIds(row.offeredOptionIds),
  };
}

function parseOfferedOptionIds(value: string | null): string[] | null {
  if (value === null) return null;
  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) && parsed.every((item) => typeof item === 'string') ? parsed : null;
  } catch {
    return null;
  }
}

function selectedOptionIdForDecision(
  request: PermissionRequest,
  decision: PermissionDecision
): string | null {
  return request.options.find(
    (option) => option.kind === `${decision}_once` || option.kind === `${decision}_always`
  )?.optionId ?? null;
}

function decisionFromOptionKind(optionKind: string): PermissionDecision {
  return optionKind.startsWith('allow') ? 'allow' : 'reject';
}

function defaultDatabasePath(): string {
  return join(app.getPath('home'), '.maenifold', 'permissions.db');
}

export class NodeSqlitePermissionLedger {
  private readonly db: DatabaseSync;
  private lastAutoDecision: LastAutoPermissionDecision | null = null;

  constructor(databasePath = defaultDatabasePath()) {
    if (databasePath !== ':memory:') {
      mkdirSync(dirname(databasePath), { recursive: true });
    }
    this.db = new DatabaseSync(databasePath);
    this.migrateAuditTable();
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS permissions (
        id INTEGER PRIMARY KEY,
        agent TEXT NOT NULL,
        scope TEXT NOT NULL,
        scope_session_id TEXT,
        tool_kind TEXT NOT NULL,
        canonical_target TEXT,
        risk_class TEXT,
        decision TEXT NOT NULL,
        created_at TEXT NOT NULL,
        last_used_at TEXT
      );
      CREATE INDEX IF NOT EXISTS idx_lookup ON permissions(agent, tool_kind, canonical_target);
      CREATE TABLE IF NOT EXISTS permission_audit (
        id INTEGER PRIMARY KEY,
        permission_id INTEGER,
        agent TEXT NOT NULL,
        session_id TEXT NOT NULL,
        tool_kind TEXT NOT NULL,
        canonical_target TEXT,
        decision TEXT,
        created_at TEXT NOT NULL,
        source TEXT,
        token_id TEXT,
        offered_option_ids TEXT,
        selected_option_id TEXT,
        rejection_reason TEXT
      );
    `);
  }

  private migrateAuditTable(): void {
    const auditCols = (
      this.db.prepare('PRAGMA table_info(permission_audit)').all() as Array<{ name: string }>
    );
    const isLegacyV1 = auditCols.length > 0 && !auditCols.some((column) => column.name === 'source');
    if (!isLegacyV1) return;
    this.db.exec(`
      BEGIN;
      CREATE TABLE permission_audit_v3 (
        id INTEGER PRIMARY KEY,
        permission_id INTEGER,
        agent TEXT NOT NULL,
        session_id TEXT NOT NULL,
        tool_kind TEXT NOT NULL,
        canonical_target TEXT,
        decision TEXT,
        created_at TEXT NOT NULL,
        source TEXT,
        token_id TEXT,
        offered_option_ids TEXT,
        selected_option_id TEXT,
        rejection_reason TEXT
      );
      INSERT INTO permission_audit_v3
        (id, permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at)
      SELECT
        id, permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at
      FROM permission_audit;
      DROP TABLE permission_audit;
      ALTER TABLE permission_audit_v3 RENAME TO permission_audit;
      COMMIT;
    `);
  }

  tryAutoDecide(
    request: PermissionRequest,
    context: PermissionLookupContext
  ): PermissionDecision | null {
    const toolKind = normalizePermissionToolKind(request.toolCall.kind);
    const canonicalTarget = canonicalPermissionTarget(request, toolKind);
    const riskClass = permissionRiskClass(request);
    const row = this.db.prepare(`
      SELECT * FROM permissions
      WHERE agent = ?
        AND tool_kind = ?
        AND (canonical_target IS ? OR canonical_target = ?)
        AND (risk_class IS ? OR risk_class = ?)
        AND (
          scope = 'forever'
          OR scope = 'agent'
          OR (scope = 'session' AND scope_session_id = ?)
        )
      ORDER BY id DESC
      LIMIT 1
    `).get(
      context.agent,
      toolKind,
      canonicalTarget,
      canonicalTarget,
      riskClass,
      riskClass,
      context.sessionId
    ) as DbRow | undefined;

    if (!row) {
      this.lastAutoDecision = null;
      return null;
    }

    const now = new Date().toISOString();
    this.db.prepare('UPDATE permissions SET last_used_at = ? WHERE id = ?').run(now, row.id);
    this.db.prepare(`
      INSERT INTO permission_audit(
        permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at,
        source, token_id, offered_option_ids, selected_option_id, rejection_reason
      )
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      row.id,
      context.agent,
      context.sessionId,
      toolKind,
      canonicalTarget,
      row.decision,
      now,
      'main_auto_decide',
      null,
      JSON.stringify(request.options.map((option) => option.optionId)),
      selectedOptionIdForDecision(request, row.decision),
      null
    );
    this.lastAutoDecision = {
      permissionId: row.id,
      agent: context.agent,
      sessionId: context.sessionId,
      toolKind,
      canonicalTarget,
      decision: row.decision,
    };
    return row.decision;
  }

  recordDecision(
    request: PermissionRequest,
    context: PermissionRecordContext
  ): PersistedPermissionDecision {
    const toolKind = normalizePermissionToolKind(request.toolCall.kind);
    const canonicalTarget = canonicalPermissionTarget(request, toolKind);
    const riskClass = permissionRiskClass(request);
    const scopeSessionId = context.scope === 'session' ? request.sessionId : null;
    const createdAt = new Date().toISOString();
    const result = this.db.prepare(`
      INSERT INTO permissions(agent, scope, scope_session_id, tool_kind, canonical_target, risk_class, decision, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      context.agent,
      context.scope,
      scopeSessionId,
      toolKind,
      canonicalTarget,
      riskClass,
      context.decision,
      createdAt
    );
    return mapDecision(this.db.prepare('SELECT * FROM permissions WHERE id = ?').get(result.lastInsertRowid) as DbRow);
  }

  recordResolveAttempt(input: ResolveAuditInput): void {
    const toolKind = input.request ? normalizePermissionToolKind(input.request.toolCall.kind) : 'unknown';
    const canonicalTarget = input.request
      ? canonicalPermissionTarget(input.request, normalizePermissionToolKind(input.request.toolCall.kind))
      : null;
    const now = new Date().toISOString();
    this.db.prepare(`
      INSERT INTO permission_audit(
        permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at,
        source, token_id, offered_option_ids, selected_option_id, rejection_reason
      )
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      input.permissionId,
      input.context.agent,
      input.context.sessionId,
      toolKind,
      canonicalTarget,
      input.decision,
      now,
      'renderer_user_choice',
      input.token,
      input.offeredOptionIds === null ? null : JSON.stringify(input.offeredOptionIds),
      input.selectedOptionId,
      input.rejectionReason
    );
  }

  listDecisions(): PersistedPermissionDecision[] {
    return (this.db.prepare('SELECT * FROM permissions ORDER BY agent, id DESC').all() as DbRow[]).map(mapDecision);
  }

  listAuditEvents(): PermissionAuditEvent[] {
    return this.db.prepare(`
      SELECT
        id,
        permission_id AS permissionId,
        agent,
        session_id AS sessionId,
        tool_kind AS toolKind,
        canonical_target AS canonicalTarget,
        decision,
        created_at AS createdAt,
        source,
        token_id AS tokenId,
        offered_option_ids AS offeredOptionIds,
        selected_option_id AS selectedOptionId,
        rejection_reason AS rejectionReason
      FROM permission_audit
      ORDER BY id DESC
      LIMIT 200
    `).all().map((row) => mapAuditEvent(row as AuditRow));
  }

  revokeDecision(id: number): void {
    this.db.prepare('DELETE FROM permissions WHERE id = ?').run(id);
  }

  resetDecisions(): void {
    this.db.prepare('DELETE FROM permissions').run();
  }

  getLastAutoDecision(): LastAutoPermissionDecision | null {
    return this.lastAutoDecision;
  }
}

let ledger: NodeSqlitePermissionLedger | null = null;

function getLedger(): NodeSqlitePermissionLedger {
  ledger ??= new NodeSqlitePermissionLedger();
  return ledger;
}

const RESET_CONFIRM_TTL_MS = 30 * 1000;

type ResolveDecisionPayload = {
  token: string;
  optionId: string;
  scope?: PermissionScope;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assertString(value: unknown, label: string): string {
  if (typeof value !== 'string') throw new Error(`invalid ${label}`);
  return value;
}

function assertOptionalScope(value: unknown): PermissionScope | undefined {
  if (value === undefined) return undefined;
  if (value !== 'session' && value !== 'agent' && value !== 'forever') {
    throw new Error('invalid scope');
  }
  return value;
}

function assertPermissionRequest(value: unknown): PermissionRequest {
  if (!isRecord(value)) throw new Error('invalid request');
  const sessionId = assertString(value.sessionId, 'request.sessionId');
  if (!isRecord(value.toolCall)) throw new Error('invalid request.toolCall');
  const toolCall = value.toolCall;
  assertString(toolCall.toolCallId, 'request.toolCall.toolCallId');
  assertString(toolCall.kind, 'request.toolCall.kind');
  if (!Array.isArray(value.options)) throw new Error('invalid request.options');
  for (const option of value.options) {
    if (!isRecord(option)) throw new Error('invalid request.options');
    assertString(option.optionId, 'request.options.optionId');
    assertString(option.kind, 'request.options.kind');
    assertString(option.name, 'request.options.name');
  }
  return { ...(value as PermissionRequest), sessionId };
}

function assertLookupContext(value: unknown): PermissionLookupContext {
  if (!isRecord(value)) throw new Error('invalid context');
  return {
    agent: assertString(value.agent, 'context.agent'),
    sessionId: assertString(value.sessionId, 'context.sessionId'),
  };
}

function assertEvaluateRequestPayload(payload: unknown): {
  request: PermissionRequest;
  context: PermissionLookupContext;
} {
  if (!isRecord(payload)) throw new Error('invalid payload');
  return {
    request: assertPermissionRequest(payload.request),
    context: assertLookupContext(payload.context),
  };
}

function selectedResponse(
  request: PermissionRequest,
  decision: PermissionDecision
): RequestPermissionResponse {
  const option = request.options.find(
    (candidate) => candidate.kind === `${decision}_once` || candidate.kind === `${decision}_always`
  );
  if (!option) {
    throw new Error(`missing ${decision} permission option`);
  }
  return {
    outcome: { outcome: 'selected', optionId: option.optionId },
    _meta: null,
  };
}

function permissionPromptPayload(request: PermissionRequest): PermissionPromptPayload {
  return {
    toolCall: request.toolCall,
    options: request.options,
  };
}

function assertResolveDecisionPayload(payload: unknown): ResolveDecisionPayload {
  if (!isRecord(payload)) throw new Error('invalid payload');
  return {
    token: assertString(payload.token, 'token'),
    optionId: assertString(payload.optionId, 'optionId'),
    scope: assertOptionalScope(payload.scope),
  };
}

function assertPositiveInteger(value: unknown, label: string): number {
  if (!Number.isInteger(value) || (value as number) <= 0) throw new Error(`invalid ${label}`);
  return value as number;
}

const decisionRegistry = createPermissionDecisionRegistry({
  tokenFactory: () => randomUUID(),
  recordDecision: (request, context) => {
    return getLedger().recordDecision(request, context);
  },
});

let resetConfirmToken: { token: string; expiresAt: number } | null = null;

export function registerPermissionLedgerHandlers(): void {
  ipcMain.handle('permission:evaluate-request', (_event, payload: unknown): PermissionEvaluateRequestResult => {
    const { request, context } = assertEvaluateRequestPayload(payload);
    const activeLedger = getLedger();
    const decision = activeLedger.tryAutoDecide(request, context);
    if (decision) {
      return { autoDecided: true, response: selectedResponse(request, decision) };
    }
    return {
      autoDecided: false,
      token: decisionRegistry.register(request, context),
      prompt: permissionPromptPayload(request),
    };
  });
  ipcMain.handle('permission:resolve-decision', (_event, payload: unknown) => {
    const input = assertResolveDecisionPayload(payload);
    try {
      const result = decisionRegistry.resolve(input);
      const decision = decisionFromOptionKind(result.selectedOption.kind);
      getLedger().recordResolveAttempt({
        request: result.request,
        context: result.context,
        token: input.token,
        offeredOptionIds: result.request.options.map((option) => option.optionId),
        selectedOptionId: input.optionId,
        decision,
        permissionId: result.persistedDecision?.id ?? null,
        rejectionReason: null,
      });
    } catch (err) {
      if (err instanceof PermissionResolveError) {
        getLedger().recordResolveAttempt({
          request: null,
          context: { agent: 'unknown', sessionId: 'unknown' },
          token: input.token,
          offeredOptionIds: null,
          selectedOptionId: input.optionId,
          decision: null,
          permissionId: null,
          rejectionReason: err.code,
        });
        throw new Error(JSON.stringify({ code: err.code, message: err.message }));
      }
      throw err;
    }
  });
  ipcMain.handle('permission:list-decisions', () => getLedger().listDecisions());
  ipcMain.handle('permission:list-audit-events', () => getLedger().listAuditEvents());
  ipcMain.handle('permission:revoke-decision', (_event, id: unknown) =>
    getLedger().revokeDecision(assertPositiveInteger(id, 'id'))
  );
  ipcMain.handle('permission:reset-confirm-prepare', () => {
    const token = randomUUID();
    resetConfirmToken = { token, expiresAt: Date.now() + RESET_CONFIRM_TTL_MS };
    return token;
  });
  ipcMain.handle('permission:reset-decisions', (_event, token: unknown) => {
    const input = assertString(token, 'token');
    if (!resetConfirmToken || resetConfirmToken.token !== input || resetConfirmToken.expiresAt < Date.now()) {
      throw new Error('invalid token');
    }
    resetConfirmToken = null;
    getLedger().resetDecisions();
  });
}
