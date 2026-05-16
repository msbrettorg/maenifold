// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import { useToolPaletteFields } from '../useToolPaletteFields';
import type { ToolPaletteEntry } from '../../lib/tool-palette-catalog';

const { callTool, connected } = vi.hoisted(() => ({
  callTool: vi.fn(),
  connected: { value: true, __v_isRef: true },
}));

vi.mock('../usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected,
    connect: vi.fn(),
    callTool,
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    listTools: vi.fn(async () => []),
    diffViews: vi.fn(() => []),
    startPolling: vi.fn(),
  }),
}));
vi.mock('../useMcpClient', () => ({ useMcpClient: () => ({ connected, connect: vi.fn(), callTool }) }));

describe('useToolPaletteFields error sanitization', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    connected.value = true;
  });

  it('redacts sensitive paths before surfacing auth setup errors', async () => {
    const raw = 'auth failed at /Users/brett/secret/file.txt tenantId: a1b2c3d4-e5f6-1234-9abc-def012345678';
    const fields = useToolPaletteFields(
      { active: true, toolName: '', agentName: '', agentConfig: {}, toolArgs: {}, canSubmit: false },
      vi.fn(),
    );
    // Let the immediate watcher's refreshCatalog() drain before setting our rejection mock
    await new Promise((r) => setTimeout(r, 0));
    const tool = ref<ToolPaletteEntry>({
      kind: 'server_state',
      key: 'msx-mcp',
      namespacedName: 'msx-mcp',
      toolName: null,
      displayName: 'msx-mcp',
      description: '',
      server: 'msx-mcp',
      status: 'error',
      error: 'login required',
      health: 'auth_required',
      inputSchema: null,
    }).value;

    fields.handleToolSelect(tool);
    callTool.mockRejectedValueOnce(new Error(raw));
    await fields.runAuthSetup();

    expect(fields.authDialogError.value).toContain('<redacted>');
    expect(fields.authDialogError.value).not.toContain('/Users/brett/secret/file.txt');
    expect(fields.authDialogError.value).not.toContain('a1b2c3d4-e5f6-1234-9abc-def012345678');
  });
});
