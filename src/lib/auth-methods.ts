import type { AuthMethod } from '@agentclientprotocol/sdk';

export interface SelectableAuthMethod {
  id: string;
  name: string;
  description?: string | null;
}

export type AppAuthMethod = SelectableAuthMethod;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function readString(
  record: Record<string, unknown>,
  key: string
): string | null {
  const value = record[key];
  return typeof value === 'string' ? value : null;
}

function hasSupportedType(record: Record<string, unknown>): boolean {
  const type = record.type;
  return type === undefined || type === 'agent';
}

function readDescription(
  record: Record<string, unknown>
): string | null | undefined {
  if (!('description' in record)) {
    return null;
  }

  const value = record.description;
  if (typeof value === 'string' || value === null) {
    return value;
  }

  return undefined;
}

export function toSelectableAuthMethod(
  method: AuthMethod | unknown
): SelectableAuthMethod | null {
  if (!isRecord(method)) {
    return null;
  }

  if (!hasSupportedType(method)) {
    return null;
  }

  const id = readString(method, 'id');
  const name = readString(method, 'name');
  const description = readDescription(method);

  if (!id || !name) {
    return null;
  }

  if (description === undefined) {
    return null;
  }

  return { id, name, description };
}

export function normalizeAuthMethods(
  methods: readonly (AuthMethod | unknown)[]
): SelectableAuthMethod[] {
  const normalizedMethods: SelectableAuthMethod[] = [];
  for (const method of methods) {
    const normalizedMethod = toSelectableAuthMethod(method);
    if (normalizedMethod) {
      normalizedMethods.push(normalizedMethod);
    }
  }
  return normalizedMethods;
}
