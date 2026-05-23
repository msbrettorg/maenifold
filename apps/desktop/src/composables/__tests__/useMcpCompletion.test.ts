// @vitest-environment jsdom
import type { PromptReference } from '@modelcontextprotocol/sdk/types';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { McpCapabilityUnsupportedError } from '../useMcpResources';
import { useMcpCompletion } from '../useMcpCompletion';

const mockState = vi.hoisted(() => ({
  current: null as ReturnType<typeof createMcpMock> | null,
}));

vi.mock('../useSharedMcpClient', () => ({
  getSharedMcpClient: vi.fn(() => mockState.current),
}));

const promptRef: PromptReference = { type: 'ref/prompt', name: 'completable-prompt' };

function createMcpMock(options: {
  connected?: boolean;
  supportsCompletion?: boolean;
  values?: string[];
  error?: Error;
} = {}) {
  const mcp = {
    connected: { value: options.connected ?? true },
    connect: vi.fn(async () => {
      mcp.connected.value = true;
    }),
    supportsCompletion: vi.fn(() => options.supportsCompletion ?? true),
    complete: vi.fn(async () => {
      if (options.error) {
        throw options.error;
      }
      const values = options.values ?? ['formal', 'friendly'];
      return {
        completion: {
          values,
          total: values.length,
        },
      };
    }),
  };
  return mcp;
}

let nextBaseUrl = 0;

function setup(options: Parameters<typeof createMcpMock>[0] = {}) {
  const mcp = createMcpMock(options);
  mockState.current = mcp;
  const completion = useMcpCompletion(`http://mcp-completion-test-${nextBaseUrl++}`);
  return { completion, mcp };
}

describe('useMcpCompletion', () => {
  afterEach(() => {
    mockState.current = null;
    vi.clearAllMocks();
  });

  it('returns completion suggestions when completions capability is advertised', async () => {
    const { completion, mcp } = setup();

    await expect(completion.complete(promptRef, 'style', 'f')).resolves.toEqual({
      completion: { values: ['formal', 'friendly'], total: 2 },
    });
    expect(mcp.complete).toHaveBeenCalledWith(promptRef, 'style', 'f');
    expect(completion.supported.value).toBe(true);
  });

  it('throws when completions capability is not advertised', async () => {
    const { completion, mcp } = setup({ supportsCompletion: false });

    await expect(completion.complete(promptRef, 'style', 'f')).rejects.toBeInstanceOf(
      McpCapabilityUnsupportedError,
    );
    expect(mcp.complete).not.toHaveBeenCalled();
    expect(completion.supported.value).toBe(false);
  });

  it('passes an empty argument value through to the MCP client', async () => {
    const { completion, mcp } = setup({ values: ['formal'] });

    await expect(completion.complete(promptRef, 'style', '')).resolves.toEqual({
      completion: { values: ['formal'], total: 1 },
    });
    expect(mcp.complete).toHaveBeenCalledWith(promptRef, 'style', '');
  });

  it('caps completion values at 100 entries', async () => {
    const values = Array.from({ length: 125 }, (_, index) => `value-${index}`);
    const { completion } = setup({ values });

    const result = await completion.complete(promptRef, 'style', 'value');

    expect(result.completion.values).toHaveLength(100);
    expect(result.completion.total).toBe(125);
    expect(result.completion.hasMore).toBe(true);
  });

  it('propagates network errors from the MCP client', async () => {
    const error = new Error('network down');
    const { completion } = setup({ error });

    await expect(completion.complete(promptRef, 'style', 'f')).rejects.toThrow('network down');
  });
});
