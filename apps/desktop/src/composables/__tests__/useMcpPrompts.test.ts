// @vitest-environment jsdom
import type { JsonRpcNotification } from '../mcpTransport';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { McpCapabilityUnsupportedError } from '../useMcpResources';
import { useMcpPrompts } from '../useMcpPrompts';

const mockState = vi.hoisted(() => ({
  current: null as ReturnType<typeof createMcpMock> | null,
}));

vi.mock('../usePipelineMcp', () => ({
  getSharedMcpClient: vi.fn(() => mockState.current),
}));

function createMcpMock(options: { connected?: boolean; supportsPrompts?: boolean } = {}) {
  const notificationHandlers = new Set<(notification: JsonRpcNotification) => void>();
  const disconnectHandlers = new Set<() => void>();
  const prompts = [{ name: 'summarize', description: 'Summarize text' }];
  const promptResult = {
    messages: [{ role: 'user' as const, content: { type: 'text' as const, text: 'Summarize: hello' } }],
  };

  const mcp = {
    connected: { value: options.connected ?? true },
    connect: vi.fn(async () => {
      mcp.connected.value = true;
    }),
    listPrompts: vi.fn(async () => prompts),
    getPrompt: vi.fn(async () => promptResult),
    supportsPrompts: vi.fn(() => options.supportsPrompts ?? true),
    onNotification: vi.fn((handler: (notification: JsonRpcNotification) => void) => {
      notificationHandlers.add(handler);
      return () => {
        notificationHandlers.delete(handler);
      };
    }),
    onDisconnect: vi.fn((handler: () => void) => {
      disconnectHandlers.add(handler);
      return () => {
        disconnectHandlers.delete(handler);
      };
    }),
    emitNotification(notification: JsonRpcNotification) {
      notificationHandlers.forEach((handler) => {
        handler(notification);
      });
    },
    emitDisconnect() {
      disconnectHandlers.forEach((handler) => {
        handler();
      });
    },
  };

  return mcp;
}

let nextBaseUrl = 0;

function setup(options: { connected?: boolean; supportsPrompts?: boolean } = {}) {
  const mcp = createMcpMock(options);
  mockState.current = mcp;
  const prompts = useMcpPrompts(`http://mcp-prompts-test-${nextBaseUrl++}`);
  return { mcp, prompts };
}

describe('useMcpPrompts', () => {
  afterEach(() => {
    mockState.current = null;
    vi.clearAllMocks();
  });

  it('listPrompts calls SDK listPrompts and returns prompts array', async () => {
    const { mcp, prompts } = setup();

    await expect(prompts.listPrompts()).resolves.toEqual([
      { name: 'summarize', description: 'Summarize text' },
    ]);
    expect(mcp.listPrompts).toHaveBeenCalledTimes(1);
  });

  it('getPrompt calls SDK getPrompt with name and arguments and returns result', async () => {
    const args = { topic: 'rust' };
    const { mcp, prompts } = setup();

    await expect(prompts.getPrompt('summarize', args)).resolves.toEqual({
      messages: [{ role: 'user', content: { type: 'text', text: 'Summarize: hello' } }],
    });
    expect(mcp.getPrompt).toHaveBeenCalledWith('summarize', args);
  });

  it('onPromptListChanged handler fires on prompts list_changed notification', () => {
    const handler = vi.fn();
    const { mcp, prompts } = setup();

    prompts.onPromptListChanged(handler);
    mcp.emitNotification({ method: 'notifications/prompts/list_changed' });

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('onPromptListChanged cleanup removes the handler', () => {
    const handler = vi.fn();
    const { mcp, prompts } = setup();

    const cleanup = prompts.onPromptListChanged(handler);
    cleanup();
    mcp.emitNotification({ method: 'notifications/prompts/list_changed' });

    expect(handler).not.toHaveBeenCalled();
  });

  it('listPrompts and getPrompt throw when prompts capability is not advertised', async () => {
    const { mcp, prompts } = setup({ supportsPrompts: false });

    await expect(prompts.listPrompts()).rejects.toBeInstanceOf(McpCapabilityUnsupportedError);
    await expect(prompts.getPrompt('summarize', {})).rejects.toBeInstanceOf(McpCapabilityUnsupportedError);
    expect(mcp.listPrompts).not.toHaveBeenCalled();
    expect(mcp.getPrompt).not.toHaveBeenCalled();
  });

  it('disconnect clears prompt list changed handlers', () => {
    const handler = vi.fn();
    const { mcp, prompts } = setup();

    prompts.onPromptListChanged(handler);
    mcp.emitDisconnect();
    mcp.emitNotification({ method: 'notifications/prompts/list_changed' });

    expect(handler).not.toHaveBeenCalled();
  });
});
