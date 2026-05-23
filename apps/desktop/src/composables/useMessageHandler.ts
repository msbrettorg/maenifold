import type { Ref } from 'vue';
import type { ContentBlock, McpAppUiMetadata, SavedSession, SessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { useMessagePersistence } from './useMessagePersistence';
import { useTitleRename } from './useTitleRename';
import { cancelOperation as cancelOperationImpl, sendPrompt as sendPromptImpl } from './useMessageHandler-prompt';
import { cancelPendingToolCalls, pushSystemMessage } from './useMessageHandler-session-core';
import { handleSessionUpdate as handleSessionUpdateImpl } from './useMessageHandler-session-update';

export interface UseMessageHandlerDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
  cancelPermission: () => void;
  resolveToolAppUi?: (toolName: string | null | undefined) => McpAppUiMetadata | null | undefined;
}

export function useMessageHandler({
  sessions,
  bridges,
  activeSessionId,
  savedSessions,
  saveSessionsToStore,
  cancelPermission,
  resolveToolAppUi,
}: UseMessageHandlerDeps) {
  const { persistSessionMessages } = useMessagePersistence({ savedSessions, saveSessionsToStore });
  const { renameSessionFromFirstPrompt } = useTitleRename({ savedSessions });

  function handleSessionUpdate(localSessionId: string, notification: SessionNotification): void {
    handleSessionUpdateImpl(localSessionId, notification, {
      sessions,
      savedSessions,
      saveSessionsToStore,
      resolveToolAppUi,
    });
  }

  async function sendPrompt(text: string, attachments: ContentBlock[] = []): Promise<boolean> {
    return await sendPromptImpl(text, attachments, {
      sessions,
      bridges,
      activeSessionId,
      persistSessionMessages,
      renameSessionFromFirstPrompt,
      cancelPermission,
      pushSystemMessage,
      cancelPendingToolCalls,
    });
  }

  async function cancelOperation(): Promise<void> {
    await cancelOperationImpl({
      sessions,
      bridges,
      activeSessionId,
      persistSessionMessages,
      renameSessionFromFirstPrompt,
      cancelPermission,
      pushSystemMessage,
      cancelPendingToolCalls,
    });
  }

  return {
    handleSessionUpdate,
    sendPrompt,
    cancelOperation,
    pushSystemMessage,
    cancelPendingToolCalls,
  };
}
