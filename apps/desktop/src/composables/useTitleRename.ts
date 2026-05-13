import type { Ref } from 'vue';
import type { SavedSession, SessionState } from '../lib/types';
import { deriveSessionTitleFromFirstMessage, isGeneratedSessionTitle } from './session-utils';

interface UseTitleRenameDeps {
  savedSessions: Ref<SavedSession[]>;
}

export function useTitleRename({ savedSessions }: UseTitleRenameDeps) {
  function renameSessionFromFirstPrompt(session: SessionState, text: string): void {
    if (!session.session || session.messages.some((message) => message.role === 'user')) {
      return;
    }

    if (!isGeneratedSessionTitle(session.session.title, session.session.agentName || session.agentName)) {
      return;
    }

    const nextTitle = deriveSessionTitleFromFirstMessage(text);
    if (!nextTitle) {
      return;
    }

    session.session.title = nextTitle;
    const savedSession = savedSessions.value.find((entry) => entry.sessionId === session.session?.sessionId);
    if (savedSession) {
      savedSession.title = nextTitle;
    }
  }

  return {
    renameSessionFromFirstPrompt,
  };
}
