import type { SessionState, StopReason } from '../lib/types';

export function pushSystemMessage(session: SessionState, content: string): void {
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'system',
    content,
    timestamp: Date.now(),
  });
}

export function cancelPendingToolCalls(session: SessionState): void {
  for (const toolCall of session.toolCalls.values()) {
    if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
      toolCall.status = 'cancelled';
    }
  }

  for (const message of session.messages) {
    if (!message.toolCalls) continue;
    for (const toolCall of message.toolCalls) {
      if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
        toolCall.status = 'cancelled';
      }
    }
  }
}

export function markLatestAssistantStopReason(session: SessionState, stopReason: StopReason): void {
  let lastUserIndex = -1;
  let lastAssistantIndex = -1;
  for (let i = session.messages.length - 1; i >= 0; i -= 1) {
    const role = session.messages[i].role;
    if (role === 'user' && lastUserIndex === -1) lastUserIndex = i;
    if (role === 'assistant' && lastAssistantIndex === -1) lastAssistantIndex = i;
    if (lastUserIndex !== -1 && lastAssistantIndex !== -1) break;
  }
  if (lastAssistantIndex > lastUserIndex && lastAssistantIndex >= 0) {
    session.messages[lastAssistantIndex].stopReason = stopReason;
    return;
  }
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: '',
    timestamp: Date.now(),
    stopReason,
  });
}

export function markAssistantInterrupted(session: SessionState): void {
  markLatestAssistantStopReason(session, 'connection_lost');
}
