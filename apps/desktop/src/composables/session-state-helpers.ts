import type { Ref } from 'vue';
import type { SessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { normalizeConfigOptions } from './session-utils';

type SessionMapRef = Ref<Map<string, SessionState>>;
type ConnectedSessionState = SessionState & {
  session: NonNullable<SessionState['session']>;
};

export interface ActiveSessionContext {
  localId: string;
  session: ConnectedSessionState;
  bridge: AcpClientBridge;
}

export function getActiveSessionContext(
  activeSessionId: Ref<string | null>,
  sessions: SessionMapRef,
  bridges: Map<string, AcpClientBridge>
): ActiveSessionContext | null {
  const localId = activeSessionId.value;
  if (!localId) return null;

  const session = sessions.value.get(localId);
  const bridge = bridges.get(localId);
  if (!session || !bridge || !session.session) return null;

  return { localId, session: session as ConnectedSessionState, bridge };
}

export function requireActiveSessionContext(
  activeSessionId: Ref<string | null>,
  sessions: SessionMapRef,
  bridges: Map<string, AcpClientBridge>
): ActiveSessionContext {
  const context = getActiveSessionContext(activeSessionId, sessions, bridges);
  if (!context) {
    throw new Error('No active session');
  }
  return context;
}

export function isActiveSessionContextCurrent(
  context: ActiveSessionContext,
  sessions: SessionMapRef,
  bridges: Map<string, AcpClientBridge>
): boolean {
  return (
    sessions.value.get(context.localId) === context.session &&
    bridges.get(context.localId) === context.bridge &&
    context.session.isConnected
  );
}

export function applyConfigOptionsUpdate(
  session: SessionState,
  configOptions: unknown
): void {
  session.hasConfigOptions = true;
  session.configOptions = normalizeConfigOptions(configOptions);
}
