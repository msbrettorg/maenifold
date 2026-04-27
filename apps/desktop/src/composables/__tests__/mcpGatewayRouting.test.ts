import { describe, expect, it } from 'vitest';
import {
  gatewayExecuteArguments,
  isGatewayPassthroughTool,
  parseGatewayCommands,
  parseGatewayDomains,
} from '../mcpGatewayRouting';
import type { ToolCallResult } from '../mcpTransport';

function toolText(payload: unknown): ToolCallResult {
  return {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  };
}

describe('mcp gateway routing helpers', () => {
  it('parses gateway index domains and commands', () => {
    expect(parseGatewayDomains(toolText({
      domains: [{ name: 'maenifold.memory' }, { name: 'gate' }],
    }))).toEqual([{ name: 'maenifold.memory' }, { name: 'gate' }]);

    expect(parseGatewayCommands(toolText({
      commands: [{ name: 'ListMemories' }, { name: 'ReadMemory' }],
    }), 'maenifold.memory')).toEqual([{ name: 'ListMemories' }, { name: 'ReadMemory' }]);
  });

  it('rejects malformed gateway index content', () => {
    expect(() => parseGatewayDomains(toolText({ commands: [] }))).toThrow(/malformed domains/);
    expect(() => parseGatewayCommands(toolText({ domains: [] }), 'maenifold.memory')).toThrow(
      /malformed commands/,
    );
  });

  it('builds exact gateway execute arguments', () => {
    expect(gatewayExecuteArguments({ domain: 'maenifold.memory', command: 'ListMemories' }, {})).toEqual({
      domain: 'maenifold.memory',
      command: 'ListMemories',
      parameters: {},
    });
  });

  it('limits direct calls to gateway control-plane and standalone tools', () => {
    expect(isGatewayPassthroughTool('GatewayIndex')).toBe(true);
    expect(isGatewayPassthroughTool('GatewayExecute')).toBe(true);
    expect(isGatewayPassthroughTool('GetHelp')).toBe(true);
    expect(isGatewayPassthroughTool('ReadMcpResource')).toBe(true);
    expect(isGatewayPassthroughTool('ListMemories')).toBe(false);
    expect(isGatewayPassthroughTool('Pipeline')).toBe(false);
  });
});
