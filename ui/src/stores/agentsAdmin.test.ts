import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

vi.mock('../lib/mcp/client', async (importActual) => {
  const actual = await importActual<typeof import('../lib/mcp/client')>();
  return { ...actual, mcpHttpClient: { callTool: vi.fn() } };
});

import { mcpHttpClient } from '../lib/mcp/client';
import type { AgentEntry, McpServerEntry } from './agentsAdmin';
import { useAgentsAdminStore } from './agentsAdmin';

function toolText(value: unknown) {
  return {
    content: [{ type: 'text', text: JSON.stringify(value) }],
  };
}

const callToolMock = vi.mocked(mcpHttpClient.callTool);

const agentEntry: AgentEntry = {
  command: 'npx',
  args: ['@github/copilot@latest', '--acp'],
  env: {
    FOO: 'bar',
  },
};

const mcpServerEntry: McpServerEntry = {
  command: 'npx',
  args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
  timeout: 10,
};

beforeEach(() => {
  setActivePinia(createPinia());
  callToolMock.mockReset();
});

describe('useAgentsAdminStore', () => {
  it('fetchConfig calls GatewayExecute/GetConfig and populates agents and mcpServers', async () => {
    const store = useAgentsAdminStore();
    const config = {
      agents: {
        foo: agentEntry,
      },
      mcpServers: {
        filesystem: mcpServerEntry,
      },
    };

    callToolMock.mockResolvedValueOnce(toolText(config));

    await store.fetchConfig();

    expect(callToolMock).toHaveBeenCalledTimes(1);
    expect(callToolMock).toHaveBeenCalledWith('GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.agents).toEqual(config.agents);
    expect(store.mcpServers).toEqual(config.mcpServers);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('addAgent sends AddAgent with config and refreshes config afterward', async () => {
    const store = useAgentsAdminStore();

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {
          foo: agentEntry,
        },
        mcpServers: {},
      })
    );

    await store.addAgent('foo', agentEntry);

    expect(callToolMock).toHaveBeenCalledTimes(2);
    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'AddAgent',
      parameters: {
        name: 'foo',
        config: agentEntry,
      },
    });
    expect(callToolMock).toHaveBeenNthCalledWith(2, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.agents).toEqual({ foo: agentEntry });
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('updateAgent sends UpdateAgent with config and refreshes config afterward', async () => {
    const store = useAgentsAdminStore();
    const updatedAgentEntry: AgentEntry = {
      ...agentEntry,
      args: ['@github/copilot@latest', '--acp', '--verbose'],
    };

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {
          foo: updatedAgentEntry,
        },
        mcpServers: {},
      })
    );

    await store.updateAgent('foo', updatedAgentEntry);

    expect(callToolMock).toHaveBeenCalledTimes(2);
    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'UpdateAgent',
      parameters: {
        name: 'foo',
        config: updatedAgentEntry,
      },
    });
    expect(callToolMock).toHaveBeenNthCalledWith(2, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.agents).toEqual({ foo: updatedAgentEntry });
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('deleteAgent sends DeleteAgent and refreshes config afterward', async () => {
    const store = useAgentsAdminStore();

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {},
        mcpServers: {},
      })
    );

    await store.deleteAgent('foo');

    expect(callToolMock).toHaveBeenCalledTimes(2);
    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'DeleteAgent',
      parameters: {
        name: 'foo',
      },
    });
    expect(callToolMock).toHaveBeenNthCalledWith(2, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.agents).toEqual({});
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('addMcpServer sends AddMcpServer with entry and refreshes config afterward', async () => {
    const store = useAgentsAdminStore();

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {},
        mcpServers: {
          bar: mcpServerEntry,
        },
      })
    );

    await store.addMcpServer('bar', mcpServerEntry);

    expect(callToolMock).toHaveBeenCalledTimes(2);
    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'AddMcpServer',
      parameters: {
        name: 'bar',
        entry: mcpServerEntry,
      },
    });
    expect(callToolMock).toHaveBeenNthCalledWith(2, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.mcpServers).toEqual({ bar: mcpServerEntry });
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('deleteMcpServer sends DeleteMcpServer and refreshes config afterward', async () => {
    const store = useAgentsAdminStore();

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {},
        mcpServers: {},
      })
    );

    await store.deleteMcpServer('bar');

    expect(callToolMock).toHaveBeenCalledTimes(2);
    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'DeleteMcpServer',
      parameters: {
        name: 'bar',
      },
    });
    expect(callToolMock).toHaveBeenNthCalledWith(2, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.mcpServers).toEqual({});
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('surfaces fetchConfig failures through error state and clears loading', async () => {
    const store = useAgentsAdminStore();

    callToolMock.mockRejectedValueOnce(new Error('daemon unreachable'));

    await expect(store.fetchConfig()).rejects.toThrow('daemon unreachable');

    expect(store.error).toBe('daemon unreachable');
    expect(store.loading).toBe(false);
  });
});
