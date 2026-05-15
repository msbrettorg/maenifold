import { describe, expect, it, vi } from 'vitest';
import { createMcpClientOps, type SdkClient } from '../../composables/mcpClientOps';

const sensitiveError = new Error(
  'HTTP 500: Internal Server Error from /etc/secret/path for /Users/brett/private/token.txt tenantId=tenant-12345',
);

describe('createMcpClientOps', () => {
  it('sanitizes transport errors before surfacing operation status', async () => {
    const setError = vi.fn();
    const client = {
      readResource: vi.fn(async () => {
        throw sensitiveError;
      }),
    } as unknown as SdkClient;
    const ops = createMcpClientOps({
      ensureConnectedSession: vi.fn(async () => client),
      reconnectSession: vi.fn(),
      setError,
    });

    await expect(ops.readResource('memory://secret')).rejects.toThrow(sensitiveError.message);

    const surfaced = setError.mock.calls.at(-1)?.[0] ?? '';
    expect(surfaced).toBe('Resource server error');
    expect(surfaced).not.toContain('/Users/');
    expect(surfaced).not.toContain('/etc/');
    expect(surfaced).not.toContain('tenant-12345');
  });
});
