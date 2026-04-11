import type { AcpMetadata } from './types';

const WINDOWS_ABSOLUTE_PATH = /^[A-Za-z]:[\\/]/;
const WINDOWS_UNC_PATH = /^\\\\/;
const ACP_EXTENSION_META_KEY = 'maenifold/extensions';

type AcpMeta = AcpMetadata;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function cloneMeta(meta: unknown): Record<string, unknown> | null {
  return isRecord(meta) ? { ...meta } : null;
}

export function normalizePermissionToolCallPayload(
  payload: unknown
): Record<string, unknown> | undefined {
  return isRecord(payload) ? { ...payload } : undefined;
}

export function withAcpMeta<T>(
  value: T,
  extraMeta?: Record<string, unknown>
): (T extends Record<string, unknown> ? T : Record<string, unknown>) & { _meta: AcpMeta } {
  const base: Record<string, unknown> = isRecord(value) ? { ...value } : {};
  const existingMeta = cloneMeta(base._meta);
  const mergedMeta =
    existingMeta || extraMeta ? { ...(existingMeta ?? {}), ...(extraMeta ?? {}) } : null;
  return {
    ...base,
    _meta: mergedMeta,
  } as (T extends Record<string, unknown> ? T : Record<string, unknown>) & { _meta: AcpMeta };
}

export function readAdvertisedAgentMethods(meta: unknown): Set<string> {
  if (!isRecord(meta)) {
    return new Set();
  }

  const extensions = meta[ACP_EXTENSION_META_KEY];
  if (!isRecord(extensions) || !Array.isArray(extensions.supportedAgentMethods)) {
    return new Set();
  }

  return new Set(
    extensions.supportedAgentMethods.filter(
      (method): method is string => typeof method === 'string'
    )
  );
}

export function withExtensionCapabilities(
  capabilities: unknown
): Record<string, unknown> & { _meta: AcpMeta } {
  const existing = isRecord(capabilities) ? { ...capabilities } : {};
  const existingMeta = cloneMeta(existing._meta);
  const existingExtensions = isRecord(existingMeta?.[ACP_EXTENSION_META_KEY])
    ? { ...(existingMeta[ACP_EXTENSION_META_KEY] as Record<string, unknown>) }
    : {};
  const supportedAgentMethods = Array.isArray(existingExtensions.supportedAgentMethods)
    ? existingExtensions.supportedAgentMethods.filter(
        (method): method is string => typeof method === 'string'
      )
    : [];

  return {
    ...existing,
    _meta: {
      ...(existingMeta ?? {}),
      [ACP_EXTENSION_META_KEY]: {
        ...existingExtensions,
        supportedAgentMethods,
      },
    },
  };
}

export function isAbsolutePath(path: string): boolean {
  return path.startsWith('/') || WINDOWS_ABSOLUTE_PATH.test(path) || WINDOWS_UNC_PATH.test(path);
}

export function assertAbsolutePath(path: string): void {
  if (!isAbsolutePath(path)) {
    throw Object.assign(new Error('Path must be absolute'), { code: -32602 });
  }
}
