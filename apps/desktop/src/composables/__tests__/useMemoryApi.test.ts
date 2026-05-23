// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { useMemoryApi } from '../useMemoryApi';

const mockCallTool = vi.fn();

vi.mock('../useSharedMcpClient', () => ({
  getSharedMcpClient: vi.fn(() => ({
    connected: ref(true),
    connect: vi.fn(),
    error: ref(null),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
    callTool: mockCallTool,
  })),
}));

describe('useMemoryApi', () => {
  beforeEach(() => {
    mockCallTool.mockReset();
  });

  it('returns the destination URI from MoveMemory arrow responses', async () => {
    mockCallTool.mockResolvedValue('Moved memory FILE: memory://research/old → memory://research/new');

    const result = await useMemoryApi().moveMemory({
      source: 'memory://research/old',
      destination: 'memory://research/new',
    });

    expect(result).toBe('memory://research/new');
  });

  it('falls back to the caller destination when MoveMemory omits an arrow URI', async () => {
    mockCallTool.mockResolvedValue('Moved memory FILE');

    const result = await useMemoryApi().moveMemory({
      source: 'memory://research/old',
      destination: 'memory://research/new',
    });

    expect(result).toBe('memory://research/new');
  });
});
