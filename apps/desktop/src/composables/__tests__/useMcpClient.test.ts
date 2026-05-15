import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import { describe, expect, it, vi } from 'vitest';
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
});
