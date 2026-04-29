import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import type { Ref } from 'vue';
import type { AcpClientBridge } from '../lib/acp-client';
import { type WritePermissionPolicy } from '../lib/acp-file-system';
import type { SessionState } from '../lib/types';

type WritePermissionPromptPayload = {
  requestId: string;
  path: string;
  sessionId?: string;
  contentPreview: string;
};

type WritePermissionDecision = 'allow_once' | 'allow_always' | 'reject';

function pushSystemMessage(session: SessionState, content: string): void {
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'system',
    content,
    timestamp: Date.now(),
  });
}

export function createWritePermissionPolicy(
  localId: string,
  sessions: Ref<Map<string, SessionState>>,
  getBridge: () => AcpClientBridge,
): WritePermissionPolicy {
  const api = typeof window === 'undefined' ? undefined : window.electronAPI;
  if (
    api &&
    typeof api.onWritePermissionPrompt === 'function' &&
    typeof api.resolveWritePermission === 'function'
  ) {
    api.onWritePermissionPrompt((payload) => {
      void handleWritePermissionPrompt(payload, localId, sessions, getBridge);
    });
  }

  return {
    currentModeId: () => sessions.value.get(localId)?.currentModeId ?? null,
    notifyPlanModeBlocked(path) {
      const session = sessions.value.get(localId);
      if (session) pushSystemMessage(session, `Write blocked by plan mode: ${path}`);
    },
  };
}

function matchesSession(
  payload: WritePermissionPromptPayload,
  localId: string,
  sessions: Ref<Map<string, SessionState>>
): boolean {
  const session = sessions.value.get(localId);
  if (!session) return false;
  if (!payload.sessionId) return true;
  return session.session?.sessionId === payload.sessionId;
}

function writePermissionRequest(
  payload: WritePermissionPromptPayload,
  sessionId: string
): RequestPermissionRequest {
  return {
    sessionId,
    toolCall: {
      toolCallId: payload.requestId,
      title: 'Write text file',
      kind: 'fs/write_text_file',
      status: 'pending',
      rawInput: { path: payload.path, content: payload.contentPreview },
      locations: [{ path: payload.path }],
    },
    options: [
      { optionId: `${payload.requestId}-allow-once`, kind: 'allow_once', name: 'Allow once' },
      { optionId: `${payload.requestId}-allow-path`, kind: 'allow_always', name: 'Always allow this path' },
      { optionId: `${payload.requestId}-reject-once`, kind: 'reject_once', name: 'Deny' },
      { optionId: `${payload.requestId}-reject-always`, kind: 'reject_always', name: 'Always deny' },
    ],
  } as unknown as RequestPermissionRequest;
}

function selectedDecision(optionId: string, requestId: string): WritePermissionDecision {
  if (optionId === `${requestId}-allow-once`) return 'allow_once';
  if (optionId === `${requestId}-allow-path`) return 'allow_always';
  return 'reject';
}

async function handleWritePermissionPrompt(
  payload: WritePermissionPromptPayload,
  localId: string,
  sessions: Ref<Map<string, SessionState>>,
  getBridge: () => AcpClientBridge
): Promise<void> {
  if (!matchesSession(payload, localId, sessions)) return;
  const api = window.electronAPI;
  if (!api?.resolveWritePermission) return;
  const session = sessions.value.get(localId);
  if (!session) return;
  if (session.currentModeId === 'plan') {
    pushSystemMessage(session, `Write blocked by plan mode: ${payload.path}`);
    await api.resolveWritePermission({ requestId: payload.requestId, decision: 'reject' });
    return;
  }

  const response = await getBridge().requestPermission(
    writePermissionRequest(payload, session.session?.sessionId ?? localId)
  );
  const decision =
    response.outcome.outcome === 'selected'
      ? selectedDecision(response.outcome.optionId, payload.requestId)
      : 'reject';
  await api.resolveWritePermission({ requestId: payload.requestId, decision });
}
