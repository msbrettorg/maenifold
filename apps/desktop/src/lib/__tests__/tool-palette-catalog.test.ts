import { describe, expect, it } from 'vitest';

import { categorizeTools, buildToolPaletteEntries, scoredSearch } from '../tool-palette-catalog';
import type { McpServerDiscoveryResult } from '../types';

describe('tool-palette-catalog', () => {
  it('ranks exact, prefix, word, and substring matches in score order', () => {
    const items = [
      { name: 'scope' },
      { name: 'ScopeStart' },
      { name: 'alpha_scope' },
      { name: 'microscope' },
    ];

    expect(
      scoredSearch(items, 'scope', (item) => [item.name]).map((entry) => entry.item.name),
    ).toEqual(['scope', 'ScopeStart', 'alpha_scope', 'microscope']);
  });

  it('surfaces synonym matches even when the literal query is absent', () => {
    const items = [
      { name: 'msx-mcp.get_account_overview', description: 'Get account overview', server: 'msx-mcp' },
      { name: 'msx-mcp.get_my_deals', description: 'Get my deals', server: 'msx-mcp' },
      { name: 'workiq.ask_work_iq', description: 'Ask work intelligence', server: 'workiq' },
    ];

    expect(
      scoredSearch(items, 'email', (item) => [item.name, item.description, item.server]).map((entry) => entry.item.name),
    ).toEqual(['workiq.ask_work_iq']);

    expect(
      scoredSearch(items, 'account', (item) => [item.name, item.description, item.server]).map((entry) => entry.item.name),
    ).toEqual(['msx-mcp.get_account_overview', 'msx-mcp.get_my_deals']);

    expect(
      scoredSearch(items, 'account data', (item) => [item.name, item.description, item.server]).map((entry) => entry.item.name),
    ).toEqual(['msx-mcp.get_account_overview', 'msx-mcp.get_my_deals']);
  });

  it('groups tools by intent before falling back to server badges', () => {
    const items = [
      {
        server: 'custom-docs',
        name: 'custom-docs.fetch_documentation',
        description: 'Fetch product documentation',
      },
      {
        server: 'custom-crm',
        name: 'custom-crm.get_account_snapshot',
        description: 'Get account overview',
      },
      {
        server: 'custom-server',
        name: 'custom-server.do_work',
        description: 'Run a bespoke action',
      },
    ];

    expect(
      categorizeTools(items, (item) => ({
        name: item.name,
        server: item.server,
        description: item.description,
      })).map((entry) => ({
        id: entry.category.id,
        tools: entry.tools.map((tool) => tool.name),
      })),
    ).toEqual([
      { id: 'crm', tools: ['custom-crm.get_account_snapshot'] },
      { id: 'docs', tools: ['custom-docs.fetch_documentation'] },
      { id: 'other', tools: ['custom-server.do_work'] },
    ]);
  });

  describe('buildToolPaletteEntries health derivation', () => {
    it('preserves auth_required for auth-pattern errors even with stale cache', () => {
      const results: McpServerDiscoveryResult[] = [
        {
          server: 'msx-mcp',
          type: 'stdio',
          status: 'error',
          error: 'login required',
          tools: [{ name: 'msx-mcp.get_accounts', description: 'cached', inputSchema: {} }],
        },
      ];

      const entries = buildToolPaletteEntries(results, { 'msx-mcp': true });
      expect(entries).toHaveLength(1);
      expect(entries[0]!.health).toBe('auth_required');
    });

    it('downgrades non-auth errors with stale cache to stale', () => {
      const results: McpServerDiscoveryResult[] = [
        {
          server: 'filesystem',
          type: 'stdio',
          status: 'error',
          error: 'connection timed out',
          tools: [{ name: 'filesystem.ReadFile', description: 'cached', inputSchema: {} }],
        },
      ];

      const entries = buildToolPaletteEntries(results, { filesystem: true });
      expect(entries).toHaveLength(1);
      expect(entries[0]!.health).toBe('stale');
    });

    it('marks unavailable when no stale cache and no auth pattern', () => {
      const results: McpServerDiscoveryResult[] = [
        {
          server: 'broken',
          type: 'stdio',
          status: 'error',
          error: 'socket refused',
          tools: [],
        },
      ];

      const entries = buildToolPaletteEntries(results, {});
      expect(entries).toHaveLength(1);
      expect(entries[0]!.kind).toBe('server_state');
      expect(entries[0]!.health).toBe('unavailable');
    });
  });
});
