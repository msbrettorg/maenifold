import { describe, expect, it } from 'vitest';
import type { Prompt } from '@modelcontextprotocol/sdk/types';
import {
  getMcpPromptCommandMeta,
  hasMcpPromptArguments,
  toMcpPromptCommands,
} from '../useMcpPromptCommands';

function prompt(name: string, description?: string, args: Prompt['arguments'] = []): Prompt {
  return { name, description, arguments: args };
}

describe('useMcpPromptCommands', () => {
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
});
