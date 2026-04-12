import { describe, expect, it, vi } from 'vitest';
import { useSessionBootstrap } from '../useSessionBootstrap';

describe('useSessionBootstrap', () => {
  it('does not advertise terminal support during initialization', async () => {
    const initialize = vi.fn().mockResolvedValue({
      protocolVersion: 1,
      authMethods: [],
      agentCapabilities: {},
    });
    const bootstrap = useSessionBootstrap({
      getAppVersion: () => '0.1.0',
      promptForAuthMethod: async () => null,
      isSessionAborted: () => false,
    });

    await bootstrap.performInitialization({
      initialize,
      disconnect: vi.fn(),
    } as never);

    const initializeRequest = initialize.mock.calls[0]?.[0];

    expect(initializeRequest).toEqual(
      expect.objectContaining({
        clientCapabilities: expect.objectContaining({
          terminal: false,
        }),
      })
    );
    expect(initializeRequest?.clientCapabilities?.fs).toBeUndefined();
  });
});
