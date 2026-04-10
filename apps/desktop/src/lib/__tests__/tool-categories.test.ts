import { describe, expect, it } from 'vitest';
import {
  getBareToolName,
  getToolCategory,
  groupToolsByCategory,
  resolveToolDefinition,
} from '../tool-categories';

describe('tool category helpers', () => {
  it('resolves bare tool names from namespaced entries', () => {
    expect(getBareToolName('maenifold.WriteMemory')).toBe('WriteMemory');
    expect(getToolCategory('maenifold.WriteMemory')?.label).toBe('Memory');
  });

  it('groups namespaced tools using bare-name categories', () => {
    expect(
      groupToolsByCategory([
        { name: 'maenifold.WriteMemory' },
        { name: 'filesystem.ReadFile' },
      ]),
    ).toEqual([
      { group: 'Memory', tools: [{ name: 'maenifold.WriteMemory' }] },
      { group: 'Other', tools: [{ name: 'filesystem.ReadFile' }] },
    ]);
  });

  it('matches bare tool definitions for namespaced selections', () => {
    expect(
      resolveToolDefinition(
        [{ name: 'WriteMemory', inputSchema: { type: 'object' } }],
        'maenifold.WriteMemory',
      ),
    ).toEqual({ name: 'WriteMemory', inputSchema: { type: 'object' } });
  });
});
