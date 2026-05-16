import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useMcpToolDiscoveryStore } from '../mcp-tool-discovery';

const { mcpClientMock, toastWarning } = vi.hoisted(() => ({
  mcpClientMock: { connected: { value: true }, connect: vi.fn(), callTool: vi.fn() },
  toastWarning: vi.fn(),
}));

vi.mock('@/composables/useToast', () => ({ toastWarning }));
vi.mock('../../composables/useMcpClient', () => ({ useMcpClient: () => mcpClientMock }));

describe('mcp tool discovery error sanitization', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mcpClientMock.connected.value = true;
  });

  it('redacts sensitive paths before surfacing discovery errors', async () => {
    const raw = 'failed reading /Users/brett/secret/file.txt tenantId: a1b2c3d4-e5f6-1234-9abc-def012345678';
    mcpClientMock.callTool.mockRejectedValueOnce(new Error(raw));

    const store = useMcpToolDiscoveryStore();
    await store.refreshTools();

    expect(store.error).toContain('<redacted>');
    expect(store.error).not.toContain('/Users/brett/secret/file.txt');
    expect(store.error).not.toContain('a1b2c3d4-e5f6-1234-9abc-def012345678');
    expect(toastWarning).toHaveBeenCalledWith(expect.stringContaining('<redacted>'));
  });
});
