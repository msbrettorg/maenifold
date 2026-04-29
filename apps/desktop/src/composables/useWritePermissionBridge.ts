import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import type { Ref } from 'vue';
import type { AcpClientBridge } from '../lib/acp-client';
import {
  setCurrentModeProvider,
  setWritePermissionGate,
  type WritePermissionDecision,
} from '../lib/acp-file-system';
import { setChatNotificationSink } from '../lib/acp-chat-notifications';
import type { SessionState } from '../lib/types';

function pushSystemMessage(session: SessionState, content: string): void {
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'system',
    content,
    timestamp: Date.now(),
  });
}

export function bindWritePermissionBridge(
  localId: string,
  sessions: Ref<Map<string, SessionState>>,
  bridge: AcpClientBridge,
): void {
  setCurrentModeProvider(() => sessions.value.get(localId)?.currentModeId ?? null);
  setChatNotificationSink(({ path }) => {
    const session = sessions.value.get(localId);
    if (session) pushSystemMessage(session, `Write blocked by plan mode: ${path}`);
  });
  setWritePermissionGate(async ({ path, content }): Promise<WritePermissionDecision> => {
    const toolCallId = `fs-write-${crypto.randomUUID()}`;
    const request = {
      sessionId: sessions.value.get(localId)?.session?.sessionId ?? localId,
      toolCall: {
        toolCallId,
        title: 'Write text file',
        kind: 'fs/write_text_file',
        status: 'pending',
        rawInput: { path, content },
        locations: [{ path }],
      },
      options: [
        { optionId: `${toolCallId}-allow-once`, kind: 'allow_once', name: 'Allow once' },
        { optionId: `${toolCallId}-allow-path`, kind: 'allow_always', name: 'Always allow this path' },
        { optionId: `${toolCallId}-reject-once`, kind: 'reject_once', name: 'Deny' },
        { optionId: `${toolCallId}-reject-always`, kind: 'reject_always', name: 'Always deny' },
      ],
    } as unknown as RequestPermissionRequest;
    const response = await bridge.requestPermission(request);
    if (response.outcome.outcome !== 'selected') return 'reject';
    if (response.outcome.optionId === `${toolCallId}-allow-once`) return 'allow';
    if (response.outcome.optionId === `${toolCallId}-allow-path`) return 'always-allow-path';
    return 'reject';
  });
}
