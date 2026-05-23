import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Prompt } from '@modelcontextprotocol/sdk/types';
import {
  getMcpPromptCommandMeta,
  hasMcpPromptArguments,
  toMcpPromptCommands,
  useMcpPromptCommands,
} from '../useMcpPromptCommands';
import { McpCapabilityUnsupportedError } from '../useMcpResources';

const mocks = vi.hoisted(() => ({
  connect: vi.fn(),
  connected: { value: true },
  listPrompts: vi.fn(),
  onPromptListChanged: vi.fn(() => () => undefined),
}));

vi.mock('../usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    connect: mocks.connect,
    connected: mocks.connected,
  }),
}));

vi.mock('../useMcpPrompts', () => ({
  useMcpPrompts: () => ({
    listPrompts: mocks.listPrompts,
    onPromptListChanged: mocks.onPromptListChanged,
  }),
}));

function prompt(name: string, description?: string, args: Prompt['arguments'] = []): Prompt {
  return { name, description, arguments: args };
}

async function flushPromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe('useMcpPromptCommands', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    mocks.connect.mockReset();
    mocks.connected.value = true;
    mocks.listPrompts.mockReset();
    mocks.onPromptListChanged.mockClear();
    mocks.onPromptListChanged.mockReturnValue(() => undefined);
  });

  it('returns an empty command list when no prompts are advertised', () => {
    expect(toMcpPromptCommands([{ serverName: 'server-everything', prompts: [] }])).toEqual([]);
  });

  it('maps a single prompt into an mcp-prefixed command', () => {
    const commands = toMcpPromptCommands([
      { serverName: 'server-everything', prompts: [prompt('simple-prompt', 'Simple prompt')] },
    ]);

    expect(commands).toEqual([
      {
        name: 'mcp.server-everything.simple-prompt',
        description: 'Simple prompt',
        input: undefined,
        _meta: {
          group: 'MCP prompts / server-everything',
          'maenifold.mcpPrompt': {
            kind: 'mcp-prompt',
            serverName: 'server-everything',
            promptName: 'simple-prompt',
            arguments: [],
          },
        },
      },
    ]);
  });

  it('keeps prompts grouped and prefixed per server', () => {
    const commands = toMcpPromptCommands([
      { serverName: 'alpha', prompts: [prompt('summarize')] },
      { serverName: 'beta', prompts: [prompt('summarize'), prompt('explain')] },
    ]);

    expect(commands.map((command) => command.name)).toEqual([
      'mcp.alpha.summarize',
      'mcp.beta.summarize',
      'mcp.beta.explain',
    ]);
    expect(commands.map((command) => command._meta?.group)).toEqual([
      'MCP prompts / alpha',
      'MCP prompts / beta',
      'MCP prompts / beta',
    ]);
  });

  it('multi source groups produce per-server commands for overlapping names', () => {
    const commands = toMcpPromptCommands([
      { serverName: 'A', prompts: [prompt('summarize')] },
      { serverName: 'B', prompts: [prompt('summarize')] },
    ]);

    expect(commands.map((command) => command.name)).toEqual([
      'mcp.A.summarize',
      'mcp.B.summarize',
    ]);
  });

  it('includes prompt argument metadata and command input hint', () => {
    const commands = toMcpPromptCommands([
      {
        serverName: 'server-everything',
        prompts: [prompt('args-prompt', 'Prompt with args', [
          { name: 'message', description: 'Message to echo', required: true },
          { name: 'count', description: 'Repeat count' },
        ])],
      },
    ]);

    expect(commands[0]?.input).toEqual({ hint: '<message> <count>' });
    expect(hasMcpPromptArguments(commands[0])).toBe(true);
    expect(getMcpPromptCommandMeta(commands[0])).toEqual({
      kind: 'mcp-prompt',
      serverName: 'server-everything',
      promptName: 'args-prompt',
      arguments: [
        { name: 'message', description: 'Message to echo', required: true },
        { name: 'count', description: 'Repeat count' },
      ],
    });
  });

  it('refresh calls listPrompts exactly once and maps returned prompts into commands', async () => {
    mocks.listPrompts.mockResolvedValue([prompt('summarize', 'from default')]);

    const client = useMcpPromptCommands();
    await flushPromises();
    await client.refresh();

    expect(mocks.listPrompts).toHaveBeenCalledTimes(2);
    expect(client.commands.value.map((command) => command.name)).toEqual(['mcp.default.summarize']);
  });

  it('refresh keeps commands empty when listPrompts returns an empty array', async () => {
    mocks.listPrompts.mockResolvedValue([]);

    const client = useMcpPromptCommands();
    await flushPromises();

    expect(mocks.listPrompts).toHaveBeenCalledTimes(1);
    expect(client.commands.value).toEqual([]);
  });

  it('refresh returns an empty command list when prompts capability is unsupported', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    mocks.listPrompts.mockRejectedValue(new McpCapabilityUnsupportedError('prompts/list unsupported'));

    const client = useMcpPromptCommands();
    await flushPromises();

    expect(client.commands.value).toEqual([]);
    expect(errorSpy).not.toHaveBeenCalled();
  });

  it('refresh logs other listPrompts errors and clears commands', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    mocks.listPrompts.mockRejectedValue(new Error('gateway unavailable'));

    const client = useMcpPromptCommands();
    await flushPromises();

    expect(client.commands.value).toEqual([]);
    expect(errorSpy).toHaveBeenCalledTimes(1);
  });
});
