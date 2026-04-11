import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { discoverMcpTools } from '../lib/tauri';
import { deriveServerHealth } from '../lib/palette-health';
import type {
  McpDiscoveredTool,
  McpServerDiscoveryResult,
  ServerHealthStatus,
} from '../lib/types';

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const useMcpToolDiscoveryStore = defineStore('mcp-tool-discovery', () => {
  const serverResults = ref<McpServerDiscoveryResult[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const hasLoadedCatalog = ref(false);
  const lastKnownTools = ref<Record<string, McpDiscoveredTool[]>>({});

  const allTools = computed<McpDiscoveredTool[]>(() =>
    serverResults.value.flatMap((result) => result.tools)
  );

  const toolsByServer = computed<Record<string, McpDiscoveredTool[]>>(() =>
    serverResults.value.reduce<Record<string, McpDiscoveredTool[]>>((acc, result) => {
      acc[result.server] = result.tools;
      return acc;
    }, {})
  );

  const hasStaleCacheByServer = computed<Record<string, boolean>>(() =>
    Object.fromEntries(
      serverResults.value.map((result) => [result.server, result.server in lastKnownTools.value]),
    )
  );

  const serverHealth = computed<Record<string, ServerHealthStatus>>(() =>
    Object.fromEntries(
      serverResults.value.map((result) => [
        result.server,
        deriveServerHealth(result, hasStaleCacheByServer.value[result.server] ?? false),
      ]),
    )
  );

  function cloneTool(tool: McpDiscoveredTool): McpDiscoveredTool {
    return {
      name: tool.name,
      description: tool.description,
      inputSchema: tool.inputSchema,
    };
  }

  function mergeServerResults(results: McpServerDiscoveryResult[]): McpServerDiscoveryResult[] {
    return results.map((result) => {
      if (result.status === 'ok') {
        const freshTools = result.tools.map(cloneTool);
        lastKnownTools.value = {
          ...lastKnownTools.value,
          [result.server]: freshTools,
        };
        return { ...result, tools: freshTools };
      }

      const staleTools = lastKnownTools.value[result.server] ?? result.tools;
      return {
        ...result,
        tools: staleTools.map(cloneTool),
      };
    });
  }

  async function refreshTools(): Promise<McpServerDiscoveryResult[]> {
    loading.value = true;
    error.value = null;
    try {
      const results = await discoverMcpTools();
      serverResults.value = mergeServerResults(results);
      hasLoadedCatalog.value = true;
      return serverResults.value;
    } catch (cause) {
      error.value = getErrorMessage(cause);
      return serverResults.value;
    } finally {
      loading.value = false;
    }
  }

  return {
    serverResults,
    allTools,
    toolsByServer,
    hasStaleCacheByServer,
    serverHealth,
    loading,
    error,
    hasLoadedCatalog,
    refreshTools,
  };
});
