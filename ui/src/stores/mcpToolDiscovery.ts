// MCP tool-discovery store. Capability-gated templates (e.g. the enterprise
// "deal preparation" pipeline) require specific MCP tools to be present in the
// connected server. This store is the single source of truth for "which tools
// exist", and it FAILS CLOSED: while loading or after an error the tool set is
// empty so gates resolve to unavailable rather than silently enabled.
//
// Discovery source is the gateway's GatewayIndex facade, NOT MCP `tools/list`.
// Under gateway consolidation the only tools surfaced by `tools/list` are the
// handful of native facade tools (GatewayIndex, GatewayExecute, …) — every
// upstream/native command is hidden behind GatewayExecute(domain, command).
// To learn which real commands are callable we enumerate domains via
// `GatewayIndex {}` and then each domain's commands via `GatewayIndex {domain}`,
// recording each command under its fully-qualified `${domain}-${command}` and
// `${domain}.${command}` forms so capability gates (which declare names like
// "msx-mcp-get_account_overview") resolve against live gateway state.
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { mcpHttpClient } from '../lib/mcp/client';
import { extractToolText, type ToolCallTextResult } from '../lib/settingsSchema';
import type { ToolDiscoveryStatus } from '../lib/templates/templateRuntime';

interface GatewayIndexDomain {
  name?: string;
}

interface GatewayIndexDomainsResponse {
  domains?: GatewayIndexDomain[];
}

interface GatewayIndexCommand {
  name?: string;
}

interface GatewayIndexCommandsResponse {
  commands?: GatewayIndexCommand[];
}

async function callGatewayIndex<T>(args: Record<string, unknown>): Promise<T> {
  const result = await mcpHttpClient.callTool<ToolCallTextResult>('GatewayIndex', args);
  const text = extractToolText(result);
  try {
    return JSON.parse(text) as T;
  } catch (error) {
    throw new Error(
      `GatewayIndex returned invalid JSON: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

export const useMcpToolDiscoveryStore = defineStore('mcpToolDiscovery', () => {
  const status = ref<ToolDiscoveryStatus>('idle');
  const error = ref<string | null>(null);
  const toolNames = ref<ReadonlySet<string>>(new Set());

  const isReady = computed(() => status.value === 'ready');

  function hasTool(name: string): boolean {
    return toolNames.value.has(name);
  }

  function hasAllTools(names: readonly string[]): boolean {
    return names.every((name) => toolNames.value.has(name));
  }

  /**
   * Discover available MCP tools. Idempotent-friendly: callers may pass
   * `{ force: true }` to re-run after a connection change. On failure the tool
   * set is cleared so capability gates remain closed.
   *
   * Enumerates the gateway: list domains, then each domain's commands, and
   * record every command under its qualified names. Any error (including a
   * single unreachable domain) fails closed — the tool set is emptied so gated
   * templates stay hidden rather than appearing on partial data.
   */
  async function discover(options: { force?: boolean } = {}): Promise<void> {
    if (status.value === 'loading') return;
    if (status.value === 'ready' && !options.force) return;

    status.value = 'loading';
    error.value = null;
    try {
      const index = await callGatewayIndex<GatewayIndexDomainsResponse>({});
      const domains = (index.domains ?? [])
        .map((domain) => domain.name)
        .filter((name): name is string => typeof name === 'string' && name.length > 0);

      const names = new Set<string>();
      for (const domain of domains) {
        const detail = await callGatewayIndex<GatewayIndexCommandsResponse>({ domain });
        for (const command of detail.commands ?? []) {
          if (typeof command.name !== 'string' || command.name.length === 0) continue;
          names.add(`${domain}-${command.name}`);
          names.add(`${domain}.${command.name}`);
        }
      }

      toolNames.value = names;
      status.value = 'ready';
    } catch (nextError) {
      toolNames.value = new Set();
      error.value = nextError instanceof Error ? nextError.message : String(nextError);
      status.value = 'error';
    }
  }

  return { status, error, toolNames, isReady, hasTool, hasAllTools, discover };
});
