import { randomUUID } from 'node:crypto';
import type { WebContents } from 'electron';

export type WritePermissionDecision = 'allow_once' | 'allow_always' | 'reject';

type PromptPayload = {
  requestId: string;
  path: string;
  sessionId?: string;
  contentPreview: string;
};

type PendingWritePermission = {
  resolve: (decision: WritePermissionDecision) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};

const PROMPT_TIMEOUT_MS = 60_000;
const CONTENT_PREVIEW_LIMIT = 4_000;
const pending = new Map<string, PendingWritePermission>();

function assertRecord(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('invalid write permission payload');
  }
  return value;
}

function assertDecision(value: unknown): WritePermissionDecision {
  if (value === 'allow_once' || value === 'allow_always' || value === 'reject') return value;
  throw new Error('invalid write permission decision');
}

function contentPreview(content: string): string {
  return content.length <= CONTENT_PREVIEW_LIMIT
    ? content
    : `${content.slice(0, CONTENT_PREVIEW_LIMIT)}…`;
}

export function requestWritePermission(
  webContents: Pick<WebContents, 'send'>,
  filePath: string,
  content: string,
  sessionId?: string
): Promise<WritePermissionDecision> {
  const requestId = randomUUID();
  const payload: PromptPayload = {
    requestId,
    path: filePath,
    contentPreview: contentPreview(content),
  };
  if (sessionId) payload.sessionId = sessionId;

  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(requestId);
      reject(new Error('write permission request timed out'));
    }, PROMPT_TIMEOUT_MS);
    pending.set(requestId, { resolve, reject, timer });
    try {
      webContents.send('acp:write-permission:prompt', payload);
    } catch (error) {
      clearTimeout(timer);
      pending.delete(requestId);
      reject(error instanceof Error ? error : new Error(String(error)));
    }
  });
}

export function resolveWritePermissionPrompt(payload: unknown): { ok: true } {
  const input = assertRecord(payload);
  const requestId = input.requestId;
  if (typeof requestId !== 'string' || requestId.length === 0) {
    throw new Error('invalid write permission requestId');
  }
  const decision = assertDecision(input.decision);
  const entry = pending.get(requestId);
  if (!entry) throw new Error('unknown write permission request');
  clearTimeout(entry.timer);
  pending.delete(requestId);
  entry.resolve(decision);
  return { ok: true };
}

export function resetWritePermissionPromptsForTest(): void {
  for (const entry of pending.values()) {
    clearTimeout(entry.timer);
    entry.reject(new Error('write permission prompt reset'));
  }
  pending.clear();
}
