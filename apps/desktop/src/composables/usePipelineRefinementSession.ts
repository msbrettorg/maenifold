import type { useSessionStore } from '../stores/session';

export interface RefinementAssistantMessage {
  content: string;
  sessionId: string;
}

export function latestAssistantMessageForRemoteSession(
  sessionStore: ReturnType<typeof useSessionStore>,
  remoteSessionId: string | null | undefined,
): RefinementAssistantMessage | null {
  const remoteId = remoteSessionId?.trim();
  if (!remoteId) return null;

  for (const session of sessionStore.sessions.values()) {
    if (session.session?.sessionId !== remoteId) continue;
    const message = [...session.messages]
      .reverse()
      .find((candidate) => candidate.role === 'assistant');
    return message ? { content: message.content, sessionId: remoteId } : null;
  }

  return null;
}
