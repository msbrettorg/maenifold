import type { Ref } from 'vue';
import type { ChatMessage, ContentBlock, SavedSession, SessionState } from '../lib/types';
import { cloneContentBlock } from './useStreamHandler';

interface UseMessagePersistenceDeps {
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
}

export function cloneMessageForPersistence(message: ChatMessage): ChatMessage {
  return {
    id: message.id,
    role: message.role,
    content: message.content,
    blocks: message.blocks?.map((block: ContentBlock) => cloneContentBlock(block)),
    thought: message.thought,
    timestamp: message.timestamp,
    stopReason: message.stopReason,
    toolCalls: message.toolCalls?.map((toolCall) => ({
      ...toolCall,
      locations: toolCall.locations?.map((location) => ({ ...location })),
      content: toolCall.content?.map((block) => ({ ...block })),
      rawInput: toolCall.rawInput ? { ...toolCall.rawInput } : undefined,
      rawOutput: toolCall.rawOutput ? { ...toolCall.rawOutput } : undefined,
    })),
  };
}

export function useMessagePersistence({ savedSessions, saveSessionsToStore }: UseMessagePersistenceDeps) {
  async function persistSessionMessages(session: SessionState): Promise<void> {
    const sessionInfo = session.session;
    if (!sessionInfo) return;
    const savedSession = savedSessions.value.find((entry) => entry.sessionId === sessionInfo.sessionId);
    if (!savedSession) return;
    savedSession.messages = session.messages.map((message) => cloneMessageForPersistence(message));
    savedSession.lastUpdated = Date.now();
    await saveSessionsToStore();
  }
  return { persistSessionMessages };
}
