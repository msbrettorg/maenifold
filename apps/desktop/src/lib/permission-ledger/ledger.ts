import type { PermissionRequest } from '../types';

export type PermissionScope = 'session' | 'agent' | 'forever';
export type PermissionDecision = 'allow' | 'reject';
type MaybePromise<T> = T | Promise<T>;

export interface PersistedPermissionDecision {
  id: number;
  agent: string;
  scope: PermissionScope;
  scopeSessionId: string | null;
  toolKind: string;
  canonicalTarget: string | null;
  riskClass: string | null;
  decision: PermissionDecision;
  createdAt: string;
  lastUsedAt: string | null;
}

export interface PermissionAuditEvent {
  id: number;
  permissionId: number | null;
  agent: string;
  sessionId: string;
  toolKind: string;
  canonicalTarget: string | null;
  decision: PermissionDecision;
  createdAt: string;
}

export interface PermissionLookupContext {
  agent: string;
  sessionId: string;
}

export interface PermissionRecordContext {
  agent: string;
  scope: PermissionScope;
  decision: PermissionDecision;
}

export interface LastAutoPermissionDecision {
  permissionId: number | null;
  agent: string;
  sessionId: string;
  toolKind: string;
  canonicalTarget: string | null;
  decision: PermissionDecision;
}

export interface PermissionLedger {
  tryAutoDecide(
    request: PermissionRequest,
    context: PermissionLookupContext
  ): MaybePromise<PermissionDecision | null>;
  recordDecision(
    request: PermissionRequest,
    context: PermissionRecordContext
  ): MaybePromise<PersistedPermissionDecision>;
  listDecisions(): MaybePromise<PersistedPermissionDecision[]>;
  listAuditEvents(): MaybePromise<PermissionAuditEvent[]>;
  revokeDecision(id: number): MaybePromise<void>;
  resetDecisions(): MaybePromise<void>;
  getLastAutoDecision?(): LastAutoPermissionDecision | null;
  consumePendingDecisionToken?(toolCallId: string): string | null;
  resolveDecision?(payload: {
    token: string;
    optionId: string;
    scope?: PermissionScope;
  }): MaybePromise<void>;
}

class NoopPermissionLedger implements PermissionLedger {
  tryAutoDecide(): PermissionDecision | null { return null; }
  recordDecision(): PersistedPermissionDecision {
    throw new Error('Permission persistence is unavailable in this runtime');
  }
  listDecisions(): PersistedPermissionDecision[] { return []; }
  listAuditEvents(): PermissionAuditEvent[] { return []; }
  revokeDecision(): void {}
  resetDecisions(): void {}
}

class IpcPermissionLedger implements PermissionLedger {
  private lastAutoDecision: LastAutoPermissionDecision | null = null;
  private readonly pendingDecisionTokens = new Map<string, string>();

  async tryAutoDecide(
    request: PermissionRequest,
    context: PermissionLookupContext
  ): Promise<PermissionDecision | null> {
    const result = await window.electronAPI?.permissionTryAutoDecide?.({ request, context });
    this.lastAutoDecision = result?.event ?? null;
    if (result?.token) {
      this.pendingDecisionTokens.set(request.toolCall.toolCallId, result.token);
    }
    return result?.decision ?? null;
  }

  recordDecision(): never {
    throw new Error('Permission decisions must be recorded through resolve-decision tokens');
  }

  consumePendingDecisionToken(toolCallId: string): string | null {
    const token = this.pendingDecisionTokens.get(toolCallId) ?? null;
    this.pendingDecisionTokens.delete(toolCallId);
    return token;
  }

  async resolveDecision(payload: {
    token: string;
    optionId: string;
    scope?: PermissionScope;
  }): Promise<void> {
    const resolveDecision = window.electronAPI?.permissionResolveDecision;
    if (!resolveDecision) {
      throw new Error('Permission resolve-decision IPC is unavailable');
    }
    await resolveDecision(payload);
  }

  async listDecisions(): Promise<PersistedPermissionDecision[]> {
    return window.electronAPI?.permissionListDecisions?.() ?? [];
  }

  async listAuditEvents(): Promise<PermissionAuditEvent[]> {
    return window.electronAPI?.permissionListAuditEvents?.() ?? [];
  }

  async revokeDecision(id: number): Promise<void> {
    await window.electronAPI?.permissionRevokeDecision?.(id);
  }

  async resetDecisions(): Promise<void> {
    const token = await window.electronAPI?.permissionResetConfirmPrepare?.();
    await window.electronAPI?.permissionResetDecisions?.(token ?? '');
  }

  getLastAutoDecision(): LastAutoPermissionDecision | null {
    return this.lastAutoDecision;
  }
}

let defaultLedger: PermissionLedger | null = null;

export function createDefaultPermissionLedger(): PermissionLedger {
  if (typeof window !== 'undefined' && window.electronAPI?.permissionTryAutoDecide) {
    return new IpcPermissionLedger();
  }
  return new NoopPermissionLedger();
}

export function getDefaultPermissionLedger(): PermissionLedger {
  if (defaultLedger) {
    return defaultLedger;
  }
  const ledger = createDefaultPermissionLedger();
  if (!(ledger instanceof NoopPermissionLedger)) {
    defaultLedger = ledger;
  }
  return ledger;
}
