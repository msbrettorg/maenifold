import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMcpClient } from '../useMcpClient';

const sensitiveError = new Error(
  'HTTP 500: Internal Server Error from /etc/secret/path for /Users/brett/private/token.txt tenantId=tenant-12345',
);

const { createSdkMcpClient } = vi.hoisted(() => ({
  createSdkMcpClient: vi.fn(),
}));

vi.mock('../mcpClientOps', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../mcpClientOps')>();
  return {
    ...actual,
    createSdkMcpClient,
  };
});

describe('useMcpClient', () => {
  beforeEach(() => {
    createSdkMcpClient.mockReset();
  });

  it('sanitizes connection errors before surfacing the alert state', async () => {
    createSdkMcpClient.mockReturnValue({
      client: {
        close: vi.fn(async () => undefined),
        connect: vi.fn(async () => {
          throw sensitiveError;
        }),
        getServerCapabilities: vi.fn(),
      },
      transport: {
        sessionId: null,
        terminateSession: vi.fn(async () => undefined),
      },
    });
    let mcpClient: ReturnType<typeof useMcpClient> | null = null;
    mount(defineComponent({
      setup() {
        mcpClient = useMcpClient('http://localhost:8080');
        return {};
      },
      template: '<div />',
    }));

    await expect(mcpClient!.connect()).rejects.toThrow(sensitiveError.message);

    const surfaced = mcpClient!.error.value ?? '';
    expect(surfaced).toBe('Resource server error');
    expect(surfaced).not.toContain('/Users/');
    expect(surfaced).not.toContain('/etc/');
    expect(surfaced).not.toContain('tenant-12345');
  });

  it('surfaces an existing client close rejection during reconnect instead of swallowing it', async () => {
    createSdkMcpClient
      .mockReturnValueOnce({
        client: {
          close: vi.fn(async () => {
            throw new Error('UPSTREAM_CAUSE_X');
          }),
          connect: vi.fn(async () => undefined),
          getServerCapabilities: vi.fn(),
        },
        transport: {
          sessionId: 'session-1',
          terminateSession: vi.fn(async () => undefined),
        },
      })
      .mockReturnValueOnce({
        client: {
          close: vi.fn(async () => undefined),
          connect: vi.fn(async () => undefined),
          getServerCapabilities: vi.fn(),
        },
        transport: {
          sessionId: 'session-2',
          terminateSession: vi.fn(async () => undefined),
        },
      });

    let mcpClient: ReturnType<typeof useMcpClient> | null = null;
    mount(defineComponent({
      setup() {
        mcpClient = useMcpClient('http://localhost:8080');
        return {};
      },
      template: '<div />',
    }));

    await expect(mcpClient!.connect()).resolves.toBeUndefined();
    await expect(mcpClient!.connect()).rejects.toThrow('UPSTREAM_CAUSE_X');
    expect(mcpClient!.error.value).toBe('UPSTREAM_CAUSE_X');
  });

  it('keeps reconnecting clean when the existing client closes successfully', async () => {
    createSdkMcpClient
      .mockReturnValueOnce({
        client: {
          close: vi.fn(async () => undefined),
          connect: vi.fn(async () => undefined),
          getServerCapabilities: vi.fn(),
        },
        transport: {
          sessionId: 'session-1',
          terminateSession: vi.fn(async () => undefined),
        },
      })
      .mockReturnValueOnce({
        client: {
          close: vi.fn(async () => undefined),
          connect: vi.fn(async () => undefined),
          getServerCapabilities: vi.fn(),
        },
        transport: {
          sessionId: 'session-2',
          terminateSession: vi.fn(async () => undefined),
        },
      });

    let mcpClient: ReturnType<typeof useMcpClient> | null = null;
    mount(defineComponent({
      setup() {
        mcpClient = useMcpClient('http://localhost:8080');
        return {};
      },
      template: '<div />',
    }));

    await expect(mcpClient!.connect()).resolves.toBeUndefined();
    await expect(mcpClient!.connect()).resolves.toBeUndefined();
    expect(mcpClient!.error.value).toBeNull();
    expect(mcpClient!.sessionId.value).toBe('session-2');
  });
});
