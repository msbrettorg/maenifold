import type { Ref } from 'vue';
import type { SavedSession, SessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import type { AuthMethod, SessionNotification } from '@agentclientprotocol/sdk';

export interface SessionRuntimeDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
  mergeSessionsFromAgent: (
    bridge: AcpClientBridge,
    agentName: string,
    cwd: string
  ) => Promise<void>;
  handleSessionUpdate: (
    localSessionId: string,
    notification: SessionNotification
  ) => void;
  promptForAuthMethod: (
    sessionId: string,
    session: SessionState,
    authMethods: AuthMethod[],
    agentName: string
  ) => Promise<string | null>;
  getAppVersion: () => string;
  clearConnectionError: () => void;
}
