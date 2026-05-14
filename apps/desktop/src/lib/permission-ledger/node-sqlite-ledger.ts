import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import {
  canonicalPermissionTarget,
  normalizePermissionToolKind,
  permissionRiskClass,
} from './equivalence-key';
import type {
  LastAutoPermissionDecision,
  PermissionAuditEvent,
  PermissionAuditSource,
  PermissionDecision,
  PermissionLookupContext,
  PermissionRecordContext,
  PermissionRejectionReason,
  PersistedPermissionDecision,
} from './ledger';
import type { PermissionRequest } from '../types';

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

export class NodeSqlitePermissionLedger {
  private readonly db: DatabaseSync;
  private lastAutoDecision: LastAutoPermissionDecision | null = null;

  constructor(databasePath: string) {
    if (databasePath !== ':memory:') mkdirSync(dirname(databasePath), { recursive: true });
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

  tryAutoDecide(request: PermissionRequest, context: PermissionLookupContext): PermissionDecision | null {
    const toolKind = normalizePermissionToolKind(request.toolCall.kind);
    const canonicalTarget = canonicalPermissionTarget(request, toolKind);
    const riskClass = permissionRiskClass(request);
    const row = this.db.prepare(`
      SELECT * FROM permissions
      WHERE agent = ? AND tool_kind = ?
        AND (canonical_target IS ? OR canonical_target = ?)
        AND (risk_class IS ? OR risk_class = ?)
        AND (scope = 'forever' OR scope = 'agent' OR (scope = 'session' AND scope_session_id = ?))
      ORDER BY id DESC LIMIT 1
    `).get(context.agent, toolKind, canonicalTarget, canonicalTarget, riskClass, riskClass, context.sessionId) as DbRow | undefined;
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

  recordDecision(request: PermissionRequest, context: PermissionRecordContext): PersistedPermissionDecision {
    const toolKind = normalizePermissionToolKind(request.toolCall.kind);
    const canonicalTarget = canonicalPermissionTarget(request, toolKind);
    const riskClass = permissionRiskClass(request);
    const createdAt = new Date().toISOString();
    const result = this.db.prepare(`
      INSERT INTO permissions(agent, scope, scope_session_id, tool_kind, canonical_target, risk_class, decision, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      context.agent,
      context.scope,
      context.scope === 'session' ? request.sessionId : null,
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
      SELECT id, permission_id AS permissionId, agent, session_id AS sessionId,
        tool_kind AS toolKind, canonical_target AS canonicalTarget, decision, created_at AS createdAt,
        source, token_id AS tokenId, offered_option_ids AS offeredOptionIds,
        selected_option_id AS selectedOptionId, rejection_reason AS rejectionReason
      FROM permission_audit ORDER BY id DESC LIMIT 200
    `).all().map((row) => mapAuditEvent(row as AuditRow));
  }

  revokeDecision(id: number): void { this.db.prepare('DELETE FROM permissions WHERE id = ?').run(id); }
  resetDecisions(): void { this.db.prepare('DELETE FROM permissions').run(); }
  getLastAutoDecision(): LastAutoPermissionDecision | null { return this.lastAutoDecision; }
}
