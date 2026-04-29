import type { PermissionRequest } from '../types';

export type PermissionToolKind =
  | 'read'
  | 'edit'
  | 'delete'
  | 'move'
  | 'search'
  | 'execute'
  | 'think'
  | 'fetch'
  | 'switch_mode'
  | 'other';

const FILE_KINDS = new Set<PermissionToolKind>(['read', 'edit', 'delete', 'move', 'search']);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringValue(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function firstString(payload: Record<string, unknown> | undefined, keys: string[]): string | null {
  if (!payload) return null;
  for (const key of keys) {
    const value = stringValue(payload[key]);
    if (value) return value;
  }
  return null;
}

export function normalizePermissionToolKind(kind: string | null | undefined): PermissionToolKind {
  const normalized = (kind ?? 'other').trim().toLowerCase();
  if (normalized === 'bash' || normalized === 'shell' || normalized === 'terminal') return 'execute';
  if (normalized === 'write' || normalized === 'fs/write_text_file') return 'edit';
  if (normalized === 'fs/read_text_file') return 'read';
  if (
    normalized === 'read' ||
    normalized === 'edit' ||
    normalized === 'delete' ||
    normalized === 'move' ||
    normalized === 'search' ||
    normalized === 'execute' ||
    normalized === 'think' ||
    normalized === 'fetch' ||
    normalized === 'switch_mode'
  ) {
    return normalized;
  }
  return 'other';
}

export function canonicalPermissionTarget(request: PermissionRequest, kind = normalizePermissionToolKind(request.toolCall.kind)): string | null {
  if (kind === 'think') return null;
  const rawInput = isRecord(request.toolCall.rawInput) ? request.toolCall.rawInput : undefined;

  if (kind === 'execute') {
    const command = firstString(rawInput, ['command', 'cmd', 'program']) ?? request.toolCall.title;
    const cwd = firstString(rawInput, ['cwd', 'workingDirectory', 'workdir']) ?? '';
    return `${command}\u0000${cwd}`;
  }

  if (kind === 'fetch') {
    const rawUrl = firstString(rawInput, ['url', 'uri']) ?? request.toolCall.locations?.[0]?.path ?? '';
    try {
      const parsed = new URL(rawUrl);
      return parsed.host;
    } catch {
      return rawUrl;
    }
  }

  if (FILE_KINDS.has(kind)) {
    return (
      firstString(rawInput, ['path', 'filePath', 'targetPath', 'sourcePath', 'destinationPath', 'uri']) ??
      request.toolCall.locations?.[0]?.path ??
      null
    );
  }

  return (
    firstString(rawInput, ['target', 'path', 'url', 'uri', 'query']) ??
    request.toolCall.locations?.[0]?.path ??
    request.toolCall.title
  );
}

export function permissionRiskClass(request: PermissionRequest): string | null {
  const meta = [request.toolCall._meta, request._meta].filter(isRecord);
  for (const entry of meta) {
    const direct = stringValue(entry.riskClass) ?? stringValue(entry.risk_class);
    if (direct) return direct;
    if (isRecord(entry.risk)) {
      const nested = stringValue(entry.risk.class) ?? stringValue(entry.risk.kind);
      if (nested) return nested;
    }
  }
  return null;
}

export function buildPermissionEquivalenceKey(
  request: PermissionRequest,
  agent: string,
  scope: string
): string {
  const toolKind = normalizePermissionToolKind(request.toolCall.kind);
  const canonicalTarget = canonicalPermissionTarget(request, toolKind);
  const riskClass = permissionRiskClass(request);
  return JSON.stringify({
    v: 1,
    agent,
    scope,
    toolKind,
    canonicalTarget,
    riskClass,
  });
}
