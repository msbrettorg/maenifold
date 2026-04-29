import crypto from 'node:crypto';

type WriteToken = {
  id: string;
  expiresAt: number;
  path: string;
  allowsAlways: boolean;
};

const TOKEN_TTL_MS = 30_000;
const tokens = new Map<string, WriteToken>();
const alwaysAllowed = new Set<string>();

export function issueWriteToken(path: string, allowsAlways = false): string {
  const id = crypto.randomUUID();
  tokens.set(id, {
    id,
    expiresAt: Date.now() + TOKEN_TTL_MS,
    path,
    allowsAlways,
  });
  return id;
}

export function consumeWriteToken(
  id: string,
  path: string
): { ok: true; allowsAlways: boolean } | { ok: false; reason: string } {
  const token = tokens.get(id);
  if (!token) return { ok: false, reason: 'unknown token' };
  tokens.delete(id);
  if (token.expiresAt < Date.now()) return { ok: false, reason: 'token expired' };
  if (token.path !== path) return { ok: false, reason: 'path mismatch' };
  return { ok: true, allowsAlways: token.allowsAlways };
}

export function isAlwaysAllowed(path: string): boolean {
  return alwaysAllowed.has(path);
}

export function rememberAlwaysAllowed(path: string): void {
  alwaysAllowed.add(path);
}

export function resetWritePermissionTokensForTest(): void {
  tokens.clear();
  alwaysAllowed.clear();
}
