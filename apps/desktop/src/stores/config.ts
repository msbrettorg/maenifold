// Agent configuration store with daemon-backed hot-reload support
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { useMcpClient } from '../composables/useMcpClient';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import type { AgentConfig, AgentsConfig } from '../lib/types';

const CONFIG_RESOURCE_URIS = [
  'config://settings',
  'config://agents',
  'config://mcp-servers',
] as const;

type ConfigResourceUri = typeof CONFIG_RESOURCE_URIS[number];

interface ReloadConfigResponse {
  config: AgentsConfig;
  reloaded?: unknown[];
  requires_restart?: unknown[];
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

function isConfigResourceUri(uri: string): uri is ConfigResourceUri {
  return CONFIG_RESOURCE_URIS.includes(uri as ConfigResourceUri);
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
  let unsubscribeConfigNotifications: (() => void) | null = null;
  const subscribedConfigResources = new Set<ConfigResourceUri>();

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
      if (!mcp.connected.value) {
        await mcp.connect();
      }
      const payload = unwrapToolData<ReloadConfigResponse>(await mcp.callTool('ReloadConfig', {}));
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
    await teardown();

    try {
      if (!mcp.connected.value) {
        await mcp.connect();
      }

      unsubscribeConfigNotifications = mcp.onNotification((notification) => {
        if (notification.method !== 'notifications/resources/updated') {
          return;
        }

        const uri = isRecord(notification.params) && typeof notification.params.uri === 'string'
          ? notification.params.uri
          : null;
        if (!uri || !isConfigResourceUri(uri)) {
          return;
        }

        void loadConfig();
      });

      for (const uri of CONFIG_RESOURCE_URIS) {
        await mcp.subscribeResource(uri);
        subscribedConfigResources.add(uri);
      }
    } catch (e) {
      await teardown();
      const message = `Config resource subscription failed: ${getErrorMessage(e)}`;
      error.value = message;
      toast.warning(message);
    }
  }

  function updateFromEvent(newConfig: AgentsConfig) {
    config.value = newConfig;
  }

  function clearError() {
    error.value = null;
  }

  async function teardown() {
    unsubscribeConfigNotifications?.();
    unsubscribeConfigNotifications = null;

    const resourcesToUnsubscribe = Array.from(subscribedConfigResources);
    subscribedConfigResources.clear();

    if (!mcp.connected.value) {
      return;
    }

    await Promise.all(
      resourcesToUnsubscribe.map(async (uri) => {
        try {
          await mcp.unsubscribeResource(uri);
        } catch {
          // Best-effort cleanup: a dropped session has already discarded server-side subscriptions.
        }
      }),
    );
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
