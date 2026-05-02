import { describe, expect, it } from 'vitest';
import { reactive, ref } from 'vue';
import { createEmptySessionState } from '../../lib/types';
import { createMockBridge } from '../../test/setup';
import { useSessionConnection } from '../useSessionConnection';

describe('useSessionConnection', () => {
  it('calls session/close before disconnect when the agent supports it', async () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    session.isConnected = true;
    session.session = {
      id: 'saved-1',
      agentName: 'OpenCode',
      sessionId: 'remote-1',
      title: 'Session',
      lastUpdated: Date.now(),
      cwd: '/repo',
    };

    const bridge = createMockBridge('agent-1');
    bridge.supportsSessionClose.mockReturnValue(true);
    const sessions = ref(new Map([[session.sessionId, session]]));
    const activeSessionId = ref<string | null>(session.sessionId);
    const connection = useSessionConnection({
      sessions,
      bridges: new Map([[session.sessionId, bridge as never]]),
      activeSessionId,
      cancelAuthResolver: () => {},
    });

    await connection.disconnect(session.sessionId);

    expect(bridge.closeSession).toHaveBeenCalledWith({ sessionId: 'remote-1' });
    expect(bridge.disconnect).toHaveBeenCalledOnce();
  });

  it('disconnects an inactive bridge before replacing the active session slot', async () => {
    const inactive = reactive(createEmptySessionState('local-1', 'Copilot', '/repo'));
    inactive.isConnected = false;
    inactive.isConnecting = true;

    const bridge = createMockBridge('agent-1');
    const sessions = ref(new Map([[inactive.sessionId, inactive]]));
    const bridges = new Map([[inactive.sessionId, bridge as never]]);
    const activeSessionId = ref<string | null>(inactive.sessionId);
    const connection = useSessionConnection({
      sessions,
      bridges,
      activeSessionId,
      cancelAuthResolver: () => {},
    });

    const { localId } = connection.createSessionSlot('OpenCode', '/repo');
    await Promise.resolve();

    expect(bridge.disconnect).toHaveBeenCalledOnce();
    expect(bridges.has(inactive.sessionId)).toBe(false);
    expect(sessions.value.has(inactive.sessionId)).toBe(false);
    expect(sessions.value.has(localId)).toBe(true);
    expect(activeSessionId.value).toBe(localId);
  });
});
