// Agent configuration store with daemon-backed hot-reload support
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import type { AgentConfig, AgentsConfig } from '../lib/types';
import { useToastStore } from './toast';

interface ConfigChangedEventPayload {
  path?: string;
  config: AgentsConfig;
}

interface ReloadConfigResponse {
  config: AgentsConfig;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const useConfigStore = defineStore('config', () => {
  const toastStore = useToastStore();
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
      const [configResponse, pathResponse] = await Promise.all([
        fetch(`${DAEMON_BASE_URL}/api/config`),
        fetch(`${DAEMON_BASE_URL}/api/config/path`),
      ]);

      if (!configResponse.ok) {
        throw new Error(`Failed to load config: HTTP ${configResponse.status}`);
      }
      if (!pathResponse.ok) {
        throw new Error(`Failed to load config path: HTTP ${pathResponse.status}`);
      }

      config.value = await configResponse.json() as AgentsConfig;
      configPath.value = await pathResponse.json() as string;
    } catch (e) {
      error.value = getErrorMessage(e);
      toastStore.addToast({ message: error.value, level: 'error' });
    } finally {
      loading.value = false;
    }
  }

  async function reload() {
    loading.value = true;
    error.value = null;
    try {
      const response = await fetch(`${DAEMON_BASE_URL}/api/config/reload`, {
        method: 'POST',
      });
      if (!response.ok) {
        throw new Error(`Failed to reload config: HTTP ${response.status}`);
      }

      const payload = await response.json() as ReloadConfigResponse;
      config.value = payload.config;
    } catch (e) {
      error.value = getErrorMessage(e);
      toastStore.addToast({ message: error.value, level: 'error' });
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
      const payload = JSON.parse((event as MessageEvent<string>).data) as ConfigChangedEventPayload;
      config.value = payload.config;
      configPath.value = payload.path ?? configPath.value;
      error.value = null;
      if (import.meta.env.DEV) {
        console.log('Config hot-reloaded:', payload.config);
      }
    });
    source.addEventListener('config-error', (event) => {
      const payload = JSON.parse((event as MessageEvent<string>).data) as { message?: string };
      error.value = payload.message ?? 'Config reload failed';
      toastStore.addToast({ message: error.value, level: 'warning' });
    });
    source.addEventListener('watch-error', (event) => {
      const payload = JSON.parse((event as MessageEvent<string>).data) as { message?: string };
      error.value = payload.message ?? 'Config watch failed';
      toastStore.addToast({ message: error.value, level: 'warning' });
    });
    source.onerror = () => {
      error.value = error.value ?? 'Config watch connection lost';
      toastStore.addToast({ message: error.value, level: 'warning' });
    };
    configWatch.value = source;
  }

  function updateFromEvent(newConfig: AgentsConfig) {
    config.value = newConfig;
  }

  function clearError() {
    error.value = null;
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
  };
});
