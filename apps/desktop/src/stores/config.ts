// Agent configuration store with daemon-backed hot-reload support
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import type { AgentConfig, AgentsConfig } from '../lib/types';

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
      const response = await fetch(`${DAEMON_BASE_URL}/api/config/reload`, {
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
          console.log('Config hot-reloaded:', payload.config); // audit-ok: no-console
        }
      } catch (e) {
        if (import.meta.env.DEV) {
          console.error('Failed to parse config-changed payload:', e); // audit-ok: no-console
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
          console.error('Failed to parse config-error payload:', e); // audit-ok: no-console
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
          console.error('Failed to parse watch-error payload:', e); // audit-ok: no-console
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
      const response = await fetch(`${DAEMON_BASE_URL}/api/config/agents`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, config: agent }),
      });
      if (!response.ok) {
        const body = await response.text().catch(() => '');
        throw new Error(body || `Failed to add agent: HTTP ${response.status}`);
      }
      await loadConfig();
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
