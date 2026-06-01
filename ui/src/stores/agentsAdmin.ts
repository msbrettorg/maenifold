import { ref } from 'vue';
import { defineStore } from 'pinia';

import { mcpHttpClient } from '../lib/mcp/client';
import {
  extractToolText,
  isPlainObject,
  type ToolCallTextResult,
} from '../lib/settingsSchema';

export interface EnvFromCommandEntry {
  command: string;
  args?: string[];
}

export interface McpServerStdioEntry {
  command: string;
  args?: string[];
  timeout?: number;
  env?: Record<string, string>;
  envFromFile?: Record<string, string>;
  envFromCommand?: Record<string, EnvFromCommandEntry>;
  gateway?: boolean;
}

export interface McpServerHttpEntry {
  url: string;
  headers?: Record<string, string>;
  auth?: 'azure-default';
  timeout?: number;
  gateway?: boolean;
}

export type McpServerEntry = McpServerStdioEntry | McpServerHttpEntry;

export interface AgentEntry {
  command: string;
  args?: string[];
  env?: Record<string, string>;
  envFromFile?: Record<string, string>;
  envFromCommand?: Record<string, EnvFromCommandEntry>;
  mcpServers?: Record<string, McpServerEntry>;
}

interface GetConfigResponse {
  agents?: Record<string, AgentEntry>;
  mcpServers?: Record<string, McpServerEntry>;
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function asRecord<T>(value: unknown): Record<string, T> {
  if (!isPlainObject(value)) {
    return {};
  }

  return Object.entries(value).reduce<Record<string, T>>((acc, [key, entry]) => {
    acc[key] = entry as T;
    return acc;
  }, {});
}

async function callGatewayCommand<T = unknown>(
  command: string,
  parameters: Record<string, unknown>
): Promise<T> {
  const result = await mcpHttpClient.callTool<ToolCallTextResult>('GatewayExecute', {
    domain: 'maenifold.system',
    command,
    parameters,
  });

  return JSON.parse(extractToolText(result)) as T;
}

export const useAgentsAdminStore = defineStore('agentsAdmin', () => {
  const agents = ref<Record<string, AgentEntry>>({});
  const mcpServers = ref<Record<string, McpServerEntry>>({});
  const loading = ref(false);
  const error = ref<string | null>(null);

  let pendingRequests = 0;

  function beginRequest(): void {
    if (pendingRequests === 0) {
      error.value = null;
    }

    pendingRequests += 1;
    loading.value = true;
  }

  function endRequest(): void {
    pendingRequests = Math.max(0, pendingRequests - 1);
    loading.value = pendingRequests > 0;
  }

  async function fetchConfig(): Promise<void> {
    beginRequest();

    try {
      const config = await callGatewayCommand<GetConfigResponse>('GetConfig', {});
      agents.value = asRecord<AgentEntry>(config.agents);
      mcpServers.value = asRecord<McpServerEntry>(config.mcpServers);
    } catch (err) {
      error.value = toErrorMessage(err);
      throw err;
    } finally {
      endRequest();
    }
  }

  async function mutateAndRefresh(
    command: string,
    parameters: Record<string, unknown>
  ): Promise<void> {
    beginRequest();

    let mutationError: unknown = null;
    let refreshError: unknown = null;

    try {
      await callGatewayCommand(command, parameters);
    } catch (err) {
      mutationError = err;
      error.value = toErrorMessage(err);
    } finally {
      endRequest();
    }

    try {
      await fetchConfig();
    } catch (err) {
      refreshError = err;
    }

    if (mutationError) {
      error.value = toErrorMessage(mutationError);
      throw mutationError;
    }

    if (refreshError) {
      throw refreshError;
    }
  }

  async function addAgent(name: string, entry: AgentEntry): Promise<void> {
    await mutateAndRefresh('AddAgent', { name, config: entry });
  }

  async function updateAgent(name: string, entry: AgentEntry): Promise<void> {
    await mutateAndRefresh('UpdateAgent', { name, config: entry });
  }

  async function deleteAgent(name: string): Promise<void> {
    await mutateAndRefresh('DeleteAgent', { name });
  }

  async function addMcpServer(name: string, entry: McpServerEntry): Promise<void> {
    await mutateAndRefresh('AddMcpServer', { name, entry });
  }

  async function updateMcpServer(name: string, entry: McpServerEntry): Promise<void> {
    await mutateAndRefresh('UpdateMcpServer', { name, entry });
  }

  async function deleteMcpServer(name: string): Promise<void> {
    await mutateAndRefresh('DeleteMcpServer', { name });
  }

  return {
    agents,
    mcpServers,
    loading,
    error,
    fetchConfig,
    addAgent,
    updateAgent,
    deleteAgent,
    addMcpServer,
    updateMcpServer,
    deleteMcpServer,
  };
});
