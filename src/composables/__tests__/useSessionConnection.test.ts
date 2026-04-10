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
});
