import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Prompt } from '@modelcontextprotocol/sdk/types';
import {
  getMcpPromptCommandMeta,
  hasMcpPromptArguments,
  toMcpPromptCommands,
  useMcpPromptCommands,
} from '../useMcpPromptCommands';

const mocks = vi.hoisted(() => ({
  listPrompts: vi.fn(),
  onPromptListChanged: vi.fn(() => () => undefined),
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

  it('refresh loads prompts from canonical prompts/list', async () => {
    mocks.listPrompts.mockResolvedValue([
      prompt('fixtureA:summarize', 'from A'),
      prompt('fixtureB:summarize', 'from B'),
    ]);

    const client = useMcpPromptCommands();
    await flushPromises();

    expect(mocks.listPrompts).toHaveBeenCalledTimes(1);
    expect(client.commands.value.map((command) => command.name)).toEqual([
      'mcp.default.fixtureA:summarize',
      'mcp.default.fixtureB:summarize',
    ]);
  });

  it('refresh falls back to an empty list when prompts/list fails', async () => {
    mocks.listPrompts.mockRejectedValue(new Error('prompt listing unavailable'));

    const client = useMcpPromptCommands();
    await flushPromises();

    expect(client.commands.value).toEqual([]);
  });

  it('refresh handles prompt parsing errors gracefully', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    mocks.listPrompts.mockResolvedValue([null] as unknown as Prompt[]);

    const client = useMcpPromptCommands();
    await flushPromises();

    expect(client.commands.value).toEqual([]);
    expect(errorSpy).toHaveBeenCalledTimes(1);
  });
});
