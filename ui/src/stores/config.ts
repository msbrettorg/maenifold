// Agent configuration store with hot-reload support
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { AgentsConfig, AgentConfig, AgentTransportKind } from '../lib/types';
import { getTransportKind } from '../lib/types';
import { restrictedTransports } from '../lib/platform';
import { getConfig, reloadConfig, getConfigPath, onConfigChanged } from '../lib/host';

export const useConfigStore = defineStore('config', () => {
  const config = ref<AgentsConfig>({ agents: {} });
  const configPath = ref<string>('');
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Stdio agents are listed in the raw config but cannot run on mobile or
  // web builds (no subprocess). Filter them out so the UI never offers an
  // option that immediately fails.
  const allAgentNames = computed(() => Object.keys(config.value.agents));

  const agentNames = computed(() => {
    if (!restrictedTransports()) return allAgentNames.value;
    return allAgentNames.value.filter(
      (name) => getTransportKind(config.value.agents[name]) !== 'stdio'
    );
  });

  const hasAgents = computed(() => agentNames.value.length > 0);

  /**
   * Preferred default agent name from `settings.defaultAgent` in
   * maenifold.json. Returns `undefined` when unset OR when the named
   * agent isn't in the agents map (so callers can fall back to first).
   * Single source of truth for the picker's auto-selection logic --
   * keeps the choice configurable without code edits.
   */
  const defaultAgentName = computed<string | undefined>(() => {
    const name = config.value.settings?.defaultAgent;
    if (!name || typeof name !== 'string') return undefined;
    return agentNames.value.includes(name) ? name : undefined;
  });

  /** Transport kind for an agent (defaults to 'stdio' for unknown names). */
  function getAgentTransportKind(name: string): AgentTransportKind {
    const c = config.value.agents[name];
    return c ? getTransportKind(c) : 'stdio';
  }

  const stdioAgentNames = computed(() =>
    allAgentNames.value.filter(
      (name) => getTransportKind(config.value.agents[name]) === 'stdio'
    )
  );

  const remoteAgentNames = computed(() =>
    allAgentNames.value.filter((name) => {
      const k = getTransportKind(config.value.agents[name]);
      return k === 'websocket' || k === 'http';
    })
  );

  async function loadConfig() {
    loading.value = true;
    error.value = null;
    try {
      config.value = await getConfig();
      configPath.value = await getConfigPath();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  async function reload() {
    loading.value = true;
    error.value = null;
    try {
      config.value = await reloadConfig();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  function getAgent(name: string): AgentConfig | undefined {
    return config.value.agents[name];
  }

  // Set up hot-reload listener
  async function setupHotReload() {
    await onConfigChanged((newConfig) => {
      config.value = newConfig;
      console.log('Config hot-reloaded:', newConfig);
    });
  }

  // Update config from event (for settings updates)
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
    allAgentNames,
    stdioAgentNames,
    remoteAgentNames,
    hasAgents,
    defaultAgentName,
    getAgentTransportKind,
    loadConfig,
    reload,
    getAgent,
    setupHotReload,
    updateFromEvent,
    clearError,
  };
});
