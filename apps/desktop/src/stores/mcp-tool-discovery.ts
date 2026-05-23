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

  function parseGatewayIndexPayload(raw: unknown, context: string): unknown {
    if (!isRecord(raw) || !Array.isArray(raw.content)) {
      return raw;
    }
    const text = raw.content[0];
    if (!isRecord(text) || typeof text.text !== 'string' || text.text.length === 0) {
      throw new Error(`GatewayIndex ${context} returned no JSON text content`);
    }
    return JSON.parse(text.text) as unknown;
  }

  function parseDomainList(raw: unknown): Array<{ name: string }> {
    const payload = parseGatewayIndexPayload(raw, 'domain list');
    const domains = isRecord(payload) ? payload.domains : null;
    if (!Array.isArray(domains)) {
      throw new Error('GatewayIndex domain list returned malformed domains payload');
    }
    return domains.flatMap((domain) => (
      isRecord(domain) && typeof domain.name === 'string' && domain.name.length > 0
        ? [{ name: domain.name }]
        : []
    ));
  }

  function buildToolBucket(domainName: string, raw: unknown): McpServerDiscoveryResult {
    const payload = parseGatewayIndexPayload(raw, `domain ${domainName}`);
    const commands = isRecord(payload) ? payload.commands : null;
    if (!Array.isArray(commands)) {
      throw new Error(`GatewayIndex domain ${domainName} returned malformed commands payload`);
    }
    return {
      server: domainName,
      type: 'gateway',
      status: 'ok',
      error: null,
      tools: commands.flatMap((command) => {
        if (!isRecord(command) || typeof command.name !== 'string' || command.name.length === 0) {
          return [];
        }
        return [{
          name: command.name,
          description: typeof command.description === 'string' ? command.description : null,
          inputSchema: command.parameters_schema,
          appUi: extractAppUiMetadata(command),
        }];
      }),
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

      const rawIndex = await mcp.callTool('GatewayIndex', {}) as unknown;
      if (controller.signal.aborted) {
        return serverResults.value;
      }
      const domains = parseDomainList(rawIndex);
      const settled = await Promise.allSettled(domains.map(async ({ name }) => {
        const rawDomain = await mcp.callTool('GatewayIndex', { domain: name, includeSchemas: true }) as unknown;
        return buildToolBucket(name, rawDomain);
      }));
      if (controller.signal.aborted) {
        return serverResults.value;
      }
      const results = settled.map((result, index) => (
        result.status === 'fulfilled'
          ? result.value
          : {
              server: domains[index]?.name ?? `gateway-${index}`,
              type: 'gateway',
              status: 'error' as const,
              error: getErrorMessage(result.reason),
              tools: [],
            }
      ));
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
