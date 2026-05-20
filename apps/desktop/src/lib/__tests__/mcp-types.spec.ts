import { describe, it, expect } from 'vitest';
import type { AgentsConfig, McpServerEntry, McpServersMap } from '../types';

const objectKeyedServers: McpServersMap = {
  workiq: {
    command: '/usr/bin/env',
    args: ['agency', 'mcp', 'workiq'],
    gateway: true,
  },
  learn: {
    url: 'https://learn.microsoft.com/api/mcp',
    gateway: true,
  },
};

const config: AgentsConfig = {
  mcpServers: objectKeyedServers,
  agents: {},
};

Object.entries(config.mcpServers ?? {}).forEach(([name, entry]: [string, McpServerEntry]) => {
  name satisfies string;
  entry satisfies McpServerEntry;
});

// @ts-expect-error mcpServers must be object-keyed by server name, not an array.
const arrayServersMustNotSatisfyMap: McpServersMap = [
  {
    command: '/usr/bin/env',
    args: ['agency', 'mcp', 'workiq'],
  },
];

void arrayServersMustNotSatisfyMap;

describe('McpServersMap type contract', () => {
  it('compiles object-keyed map and rejects array literal at type level', () => {
    expect(Object.keys(objectKeyedServers)).toEqual(['workiq', 'learn']);
    expect(arrayServersMustNotSatisfyMap.length).toBe(1);
  });
});
