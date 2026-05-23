import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { setNotificationSink, resetNotificationSink } from '@/lib/notification-sink';
import { useMcpToolDiscoveryStore } from '../mcp-tool-discovery';

const { mcpClientMock } = vi.hoisted(() => ({
  mcpClientMock: { connected: { value: true }, connect: vi.fn(), callTool: vi.fn() },
}));

const mockSink = {
  success: vi.fn(),
  info: vi.fn(),
  warning: vi.fn(),
  error: vi.fn(),
  critical: vi.fn(),
};

vi.mock('../../composables/useMcpClient', () => ({ useMcpClient: () => mcpClientMock }));

describe('mcp tool discovery error sanitization', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mcpClientMock.connected.value = true;
    setNotificationSink(mockSink);
  });

  afterEach(() => {
    resetNotificationSink();
  });

  it('redacts sensitive paths before surfacing discovery errors', async () => {
    const raw = 'failed reading /Users/brett/secret/file.txt tenantId: a1b2c3d4-e5f6-1234-9abc-def012345678';
    mcpClientMock.callTool.mockRejectedValueOnce(new Error(raw));

    const store = useMcpToolDiscoveryStore();
    await store.refreshTools();

    expect(store.error).toContain('<redacted>');
    expect(store.error).not.toContain('/Users/brett/secret/file.txt');
    expect(store.error).not.toContain('a1b2c3d4-e5f6-1234-9abc-def012345678');
    expect(mockSink.warning).toHaveBeenCalledWith(expect.stringContaining('<redacted>'));
  });
});
