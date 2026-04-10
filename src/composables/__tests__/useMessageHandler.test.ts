// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { reactive, ref } from 'vue';
import { createEmptySessionState } from '../../lib/types';
import type { SavedSession } from '../../lib/types';
import { useMessageHandler } from '../useMessageHandler';

describe('useMessageHandler', () => {
  it('stores usage_update payloads on session state', () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    const sessions = ref(new Map([[session.sessionId, session]]));
    const handler = useMessageHandler({
      sessions,
      bridges: new Map(),
      activeSessionId: ref(session.sessionId),
      savedSessions: ref<SavedSession[]>([]),
      saveSessionsToStore: async () => {},
      cancelPermission: () => {},
    });

    handler.handleSessionUpdate(session.sessionId, {
      sessionId: 'remote-1',
      update: {
        sessionUpdate: 'usage_update',
        inputTokens: 42,
        outputTokens: 7,
        cacheWriteTokens: 3,
      },
    } as never);

    expect(session.usage).toEqual({
      inputTokens: 42,
      outputTokens: 7,
      cacheWriteTokens: 3,
    });
  });
});
