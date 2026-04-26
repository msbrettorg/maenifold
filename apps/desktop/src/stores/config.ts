// Agent configuration store with daemon-backed hot-reload support
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { useMcpClient } from '../composables/useMcpClient';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import { DAEMON_BASE_URL, authenticatedFetch } from '../lib/daemon-url';
import type { AgentConfig, AgentsConfig } from '../lib/types';

interface ConfigChangedEventPayload {
  path?: string;
  config: AgentsConfig;
}

interface ReloadConfigResponse {
  config: AgentsConfig;
}

interface GetConfigPaths {
  configPath?: unknown;
  maenifoldRoot?: unknown;
}

interface GetConfigResponse {
  agents?: unknown;
  mcpServers?: unknown;
  paths?: GetConfigPaths;
  configPath?: unknown;
  path?: unknown;
  settings?: unknown;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeAgentCommand(command: unknown, args: unknown): string {
  const binary = typeof command === 'string' ? command : '';
  if (!Array.isArray(args)) {
    return binary;
  }

  const parts = args.filter((arg): arg is string => typeof arg === 'string' && arg.length > 0);
  return parts.length > 0 ? [binary, ...parts].filter(Boolean).join(' ') : binary;
}

function normalizeAgentEntry(entry: Record<string, unknown>): AgentConfig {
  const args = entry.args;
  const agentFields = Object.fromEntries(
    Object.entries(entry).filter(([key]) => key !== 'name' && key !== 'args'),
  );
  return {
    ...agentFields,
    command: normalizeAgentCommand(agentFields.command, args),
  } as AgentConfig;
}

function normalizeAgentsConfig(agents: unknown): Record<string, AgentConfig> {
  if (Array.isArray(agents)) {
    return agents.reduce<Record<string, AgentConfig>>((normalized, entry) => {
      if (!isRecord(entry) || typeof entry.name !== 'string' || entry.name.length === 0) {
        return normalized;
      }

      normalized[entry.name] = normalizeAgentEntry(entry);
      return normalized;
    }, {});
  }

  if (isRecord(agents)) {
    return Object.entries(agents).reduce<Record<string, AgentConfig>>((normalized, [name, entry]) => {
      if (!isRecord(entry)) {
        return normalized;
      }

      normalized[name] = normalizeAgentEntry({ name, ...entry });
      return normalized;
    }, {});
  }

  return {};
}

function resolveConfigPath(response: GetConfigResponse): string {
  if (typeof response.configPath === 'string') {
    return response.configPath;
  }

  if (typeof response.path === 'string') {
    return response.path;
  }

  if (typeof response.paths?.configPath === 'string') {
    return response.paths.configPath;
  }

  if (typeof response.paths?.maenifoldRoot === 'string') {
    const root = response.paths.maenifoldRoot;
    return root.endsWith('/') || root.endsWith('\\') ? `${root}maenifold.json` : `${root}/maenifold.json`;
  }

  return '';
}

function normalizeGetConfigResponse(result: unknown): { config: AgentsConfig; configPath: string } {
  const data = unwrapToolData<GetConfigResponse>(result);
  if (!isRecord(data)) {
    throw new Error('GetConfig returned an invalid response');
  }

  const config: AgentsConfig = {
    mcpServers: Array.isArray(data.mcpServers)
      ? data.mcpServers as AgentsConfig['mcpServers']
      : [],
    agents: normalizeAgentsConfig(data.agents),
  };

  if (data.settings !== undefined) {
    config.settings = data.settings as AgentsConfig['settings'];
  }

  return {
    config,
    configPath: resolveConfigPath(data),
  };
}

export const useConfigStore = defineStore('config', () => {
  const mcp = useMcpClient();
  const config = ref<AgentsConfig>({ mcpServers: [], agents: {} });
  const configPath = ref<string>('');
  const loading = ref(false);
  const error = ref<string | null>(null);
  const configWatch = ref<EventSource | null>(null);

  const agentNames = computed(() => Object.keys(config.value.agents));
  const hasAgents = computed(() => agentNames.value.length > 0);

  async function loadConfig() {
    loading.value = true;
    error.value = null;
    try {
      if (!mcp.connected.value) {
        await mcp.connect();
      }
      const payload = normalizeGetConfigResponse(await mcp.callTool('GetConfig', {}));
      config.value = payload.config;
      configPath.value = payload.configPath;
    } catch (e) {
      const message = getErrorMessage(e);
      error.value = message;
      toast.error(message);
    } finally {
      loading.value = false;
    }
  }

  async function reload() {
    loading.value = true;
    error.value = null;
    try {
      const response = await authenticatedFetch(`${DAEMON_BASE_URL}/api/config/reload`, {
        method: 'POST',
      });
      if (!response.ok) {
        throw new Error(`Failed to reload config: HTTP ${response.status}`);
      }

      const payload = await response.json() as ReloadConfigResponse;
      config.value = payload.config;
    } catch (e) {
      const message = getErrorMessage(e);
      error.value = message;
      toast.error(message);
    } finally {
      loading.value = false;
    }
  }

  function getAgent(name: string): AgentConfig | undefined {
    return config.value.agents[name];
  }

  async function setupHotReload() {
    configWatch.value?.close();

    const source = new EventSource(`${DAEMON_BASE_URL}/api/config/watch`);
    source.addEventListener('config-changed', (event) => {
      try {
        const payload = JSON.parse((event as MessageEvent<string>).data) as ConfigChangedEventPayload;
        config.value = payload.config;
        configPath.value = payload.path ?? configPath.value;
        error.value = null;
        if (import.meta.env.DEV) {
        }
      } catch (e) {
        if (import.meta.env.DEV) {
        }
        return;
      }
    });
    source.addEventListener('config-error', (event) => {
      try {
        const payload = JSON.parse((event as MessageEvent<string>).data) as { message?: string };
        const message = payload.message ?? 'Config reload failed';
        error.value = message;
        toast.warning(message);
      } catch (e) {
        if (import.meta.env.DEV) {
        }
        return;
      }
    });
    source.addEventListener('watch-error', (event) => {
      try {
        const payload = JSON.parse((event as MessageEvent<string>).data) as { message?: string };
        const message = payload.message ?? 'Config watch failed';
        error.value = message;
        toast.warning(message);
      } catch (e) {
        if (import.meta.env.DEV) {
        }
        return;
      }
    });
    source.onerror = () => {
      const message = error.value ?? 'Config watch connection lost';
      error.value = message;
      toast.warning(message);
    };
    configWatch.value = source;
  }

  function updateFromEvent(newConfig: AgentsConfig) {
    config.value = newConfig;
  }

  function clearError() {
    error.value = null;
  }

  function teardown() {
    configWatch.value?.close();
    configWatch.value = null;
  }

  async function addAgent(name: string, agent: AgentConfig): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      if (!mcp.connected.value) {
        await mcp.connect();
      }
      config.value = unwrapToolData<AgentsConfig>(
        await mcp.callTool('AddAgent', { name, config: agent }),
      );
    } catch (e) {
      const message = getErrorMessage(e);
      error.value = message;
      throw new Error(message);
    } finally {
      loading.value = false;
    }
  }

  return {
    config,
    configPath,
    loading,
    error,
    agentNames,
    hasAgents,
    loadConfig,
    reload,
    getAgent,
    setupHotReload,
    updateFromEvent,
    clearError,
    addAgent,
    teardown,
  };
});
