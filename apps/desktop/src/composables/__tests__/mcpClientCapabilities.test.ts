import { describe, expect, it, vi } from 'vitest';
import { createSdkMcpClient } from '../mcpClientOps';

describe('createSdkMcpClient', () => {
  it('advertises elicitation capability to MCP servers', () => {
    const { client } = createSdkMcpClient({
      mcpUrl: 'http://localhost:8080/mcp',
      notificationRouter: {
        dispatch: vi.fn(),
        onNotification: vi.fn(() => () => undefined),
      },
      onError: vi.fn(),
      onClose: vi.fn(),
    });

    expect((client as any)._capabilities).toHaveProperty('elicitation');
  });
});
