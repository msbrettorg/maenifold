import { getNotificationSink } from '@/lib/notification-sink';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useMcpClient } from '../composables/useMcpClient';
import { deriveServerHealth } from '../lib/palette-health';
import { sanitizeResourceErrorMessage, warnRawResourceErrorInDev } from '../lib/error-sanitizer';
import type {
  McpDiscoveredTool,
  McpServerDiscoveryResult,
  ServerHealthStatus,
} from '../lib/types';
import { extractAppUiMetadata, isRecord } from './agent-capabilities-helpers';

function getErrorMessage(error: unknown): string {
  const rawError = error instanceof Error ? error.message : String(error);
  warnRawResourceErrorInDev(rawError);
  return sanitizeResourceErrorMessage(rawError);
}

export const useMcpToolDiscoveryStore = defineStore('mcp-tool-discovery', () => {
  const mcp = useMcpClient();
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
      appUi: tool.appUi
        ? {
            resourceUri: tool.appUi.resourceUri,
            originatingServer: tool.appUi.originatingServer,
            csp: tool.appUi.csp,
            permissions: tool.appUi.permissions ? [...tool.appUi.permissions] : undefined,
          }
        : null,
    };
  }

  function normalizeTool(raw: unknown): McpDiscoveredTool | null {
    if (!isRecord(raw)) {
      return null;
    }

    const name = raw.name;
    const inputSchema = raw.inputSchema;
    if (typeof name !== 'string' || !isRecord(inputSchema)) {
      return null;
    }

    return {
      name,
      description: typeof raw.description === 'string' ? raw.description : null,
      inputSchema: { ...inputSchema },
      appUi: extractAppUiMetadata(raw),
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

  function inferServerName(toolName: string): string {
    const separatorIndex = toolName.indexOf('.');
    return separatorIndex > 0 ? toolName.slice(0, separatorIndex) : 'default';
  }

  function buildResultsFromToolList(rawTools: unknown[]): McpServerDiscoveryResult[] {
    const toolsByServerName = new Map<string, McpDiscoveredTool[]>();
    for (const rawTool of rawTools) {
      const tool = normalizeTool(rawTool);
      if (!tool) continue;
      const serverName = inferServerName(tool.name);
      const serverTools = toolsByServerName.get(serverName) ?? [];
      serverTools.push(tool);
      toolsByServerName.set(serverName, serverTools);
    }
    return Array.from(toolsByServerName.entries()).map(([server, tools]) => ({
      server,
      type: 'session',
      status: 'ok',
      error: null,
      tools,
    }));
  }

  function buildErrorResults(message: string): McpServerDiscoveryResult[] {
    return Object.entries(lastKnownTools.value).map(([server, tools]) => ({
      server,
      type: 'session',
      status: 'error',
      error: message,
      tools: tools.map(cloneTool),
    }));
  }

  let activeAbortController: AbortController | null = null;

  function cancelInflightRefresh(): void {
    if (activeAbortController) {
      activeAbortController.abort();
      activeAbortController = null;
    }
  }

  async function refreshTools(): Promise<McpServerDiscoveryResult[]> {
    // Cancel any in-flight refresh before issuing a new one.
    cancelInflightRefresh();
    const controller = new AbortController();
    activeAbortController = controller;

    loading.value = true;
    error.value = null;
    try {
      if (!mcp.connected.value) {
        await mcp.connect();
      }

      const listedTools = await mcp.listTools();
      if (controller.signal.aborted) {
        return serverResults.value;
      }
      const results = buildResultsFromToolList(listedTools);
      serverResults.value = mergeServerResults(results);
      hasLoadedCatalog.value = true;
      return serverResults.value;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === 'AbortError') {
        // Aborted refreshes are expected on unmount/cancel; do not surface as errors.
        return serverResults.value;
      }
      const message = getErrorMessage(cause);
      error.value = message;
      getNotificationSink().warning(message);
      serverResults.value = mergeServerResults(buildErrorResults(message));
      return serverResults.value;
    } finally {
      if (activeAbortController === controller) {
        activeAbortController = null;
      }
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
    cancelInflightRefresh,
  };
});
