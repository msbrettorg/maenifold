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
  PermissionDecision,
  PermissionLookupContext,
  PermissionRecordContext,
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

export class NodeSqlitePermissionLedger {
  private readonly db: DatabaseSync;
  private lastAutoDecision: LastAutoPermissionDecision | null = null;

  constructor(databasePath: string) {
    if (databasePath !== ':memory:') mkdirSync(dirname(databasePath), { recursive: true });
    this.db = new DatabaseSync(databasePath);
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
        decision TEXT NOT NULL,
        created_at TEXT NOT NULL
      );
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
      INSERT INTO permission_audit(permission_id, agent, session_id, tool_kind, canonical_target, decision, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `).run(row.id, context.agent, context.sessionId, toolKind, canonicalTarget, row.decision, now);
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

  listDecisions(): PersistedPermissionDecision[] {
    return (this.db.prepare('SELECT * FROM permissions ORDER BY agent, id DESC').all() as DbRow[]).map(mapDecision);
  }

  listAuditEvents(): PermissionAuditEvent[] {
    return this.db.prepare(`
      SELECT id, permission_id AS permissionId, agent, session_id AS sessionId,
        tool_kind AS toolKind, canonical_target AS canonicalTarget, decision, created_at AS createdAt
      FROM permission_audit ORDER BY id DESC LIMIT 200
    `).all() as unknown as PermissionAuditEvent[];
  }

  revokeDecision(id: number): void { this.db.prepare('DELETE FROM permissions WHERE id = ?').run(id); }
  resetDecisions(): void { this.db.prepare('DELETE FROM permissions').run(); }
  getLastAutoDecision(): LastAutoPermissionDecision | null { return this.lastAutoDecision; }
}
