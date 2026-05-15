const REDACTED = '<redacted>';

const FILE_URI_PATTERN = /\bfile:\/\/\/[^\s"'<>),;]+/gi;
const WINDOWS_ABSOLUTE_PATH_PATTERN = /\b[A-Za-z]:\\(?:[^\\\s"'<>),;]+\\)*[^\\\s"'<>),;]+/g;
const UNIX_ABSOLUTE_PATH_PATTERN = /(^|[\s(["'=:])\/(?:[^/\s"'<>),;]+\/)*[^/\s"'<>),;]+/g;
const TENANT_IDENTIFIER_PATTERN =
  /\b(tenant(?:Id| ID| id)?\s*[:=]\s*)[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b/gi;

export function sanitizeResourceErrorMessage(rawMessage: string): string {
  const friendlyStatusMessage = mapHttpStatusMessage(rawMessage);
  if (friendlyStatusMessage) return friendlyStatusMessage;

  const sanitized = rawMessage
    .replace(FILE_URI_PATTERN, REDACTED)
    .replace(WINDOWS_ABSOLUTE_PATH_PATTERN, REDACTED)
    .replace(UNIX_ABSOLUTE_PATH_PATTERN, (_match, prefix: string) => `${prefix}${REDACTED}`)
    .replace(TENANT_IDENTIFIER_PATTERN, (_match, prefix: string) => `${prefix}${REDACTED}`)
    .replace(/\s{2,}/g, ' ')
    .trim();

  return sanitized || 'Resource preview failed';
}

export function warnRawResourceErrorInDev(rawMessage: string): void {
  if (!import.meta.env.DEV) return;
  globalThis['console'].warn(rawMessage);
}

function mapHttpStatusMessage(message: string): string | null {
  if (/\b404\b/.test(message)) return 'Resource not found';
  if (/\b403\b/.test(message)) return 'Permission denied';
  if (/\b5\d{2}\b/.test(message)) return 'Resource server error';
  return null;
}
