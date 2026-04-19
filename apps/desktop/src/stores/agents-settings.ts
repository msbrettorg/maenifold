import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';
import { readLastUsedAgent, writeLastUsedAgent } from '../lib/preferred-agent';

const STORAGE_KEY = 'agentsSettings';
const LEGACY_MODEL_PREFERENCE_KEY = 'chatPreferredModelByAgent';

export interface AgentsSettingsState {
  defaults: {
    defaultAgent: string;
    preferredModelByAgent: Record<string, string>;
  };
  reconnectRequired: boolean;
}

function createDefaultState(): AgentsSettingsState {
  return {
    defaults: {
      defaultAgent: '',
      preferredModelByAgent: {},
    },
    reconnectRequired: false,
  };
}

function mergeState(saved: unknown): AgentsSettingsState {
  const defaults = createDefaultState();
  if (!saved || typeof saved !== 'object') {
    return defaults;
  }

  const value = saved as Partial<AgentsSettingsState>;
  return {
    defaults: {
      defaultAgent: value.defaults?.defaultAgent ?? defaults.defaults.defaultAgent,
      preferredModelByAgent: value.defaults?.preferredModelByAgent ?? defaults.defaults.preferredModelByAgent,
    },
    reconnectRequired: value.reconnectRequired ?? defaults.reconnectRequired,
  };
}

export const useAgentsSettingsStore = defineStore('agentsSettings', () => {
  const settings = ref<AgentsSettingsState>(createDefaultState());
  const initialized = ref(false);

  watch(
    settings,
    (next) => {
      if (!initialized.value) {
        return;
      }
      writeStorageValue(PREFS_STORE_PATH, STORAGE_KEY, next);
      writeStorageValue(PREFS_STORE_PATH, LEGACY_MODEL_PREFERENCE_KEY, next.defaults.preferredModelByAgent);
    },
    { deep: true },
  );

  function ensureDefaultAgent(agentNames: string[]) {
    if (agentNames.length === 0) {
      settings.value.defaults.defaultAgent = '';
      return;
    }

    if (settings.value.defaults.defaultAgent && agentNames.includes(settings.value.defaults.defaultAgent)) {
      return;
    }

    const fromStorage = readLastUsedAgent();
    const nextAgent = fromStorage && agentNames.includes(fromStorage) ? fromStorage : agentNames[0];
    settings.value.defaults.defaultAgent = nextAgent;
    writeLastUsedAgent(nextAgent);
  }

  function init(agentNames: string[] = []) {
    settings.value = mergeState(readStorageValue<AgentsSettingsState>(PREFS_STORE_PATH, STORAGE_KEY));
    if (Object.keys(settings.value.defaults.preferredModelByAgent).length === 0) {
      settings.value.defaults.preferredModelByAgent = readStorageValue<Record<string, string>>(
        PREFS_STORE_PATH,
        LEGACY_MODEL_PREFERENCE_KEY,
      ) ?? {};
    }
    ensureDefaultAgent(agentNames);
    initialized.value = true;
  }

  function setDefaultAgent(agentName: string) {
    const normalized = agentName.trim();
    settings.value.defaults.defaultAgent = normalized;
    if (normalized) {
      writeLastUsedAgent(normalized);
    }
  }

  function setPreferredModel(agentName: string, model: string) {
    const normalizedAgent = agentName.trim();
    if (!normalizedAgent) {
      return;
    }

    const normalizedModel = model.trim();
    if (!normalizedModel) {
      const { [normalizedAgent]: _removed, ...rest } = settings.value.defaults.preferredModelByAgent;
      settings.value.defaults.preferredModelByAgent = rest;
      return;
    }

    settings.value.defaults.preferredModelByAgent = {
      ...settings.value.defaults.preferredModelByAgent,
      [normalizedAgent]: normalizedModel,
    };
  }

  function markReconnectRequired() {
    settings.value.reconnectRequired = true;
  }

  function markReconnected() {
    settings.value.reconnectRequired = false;
  }

  return {
    settings,
    init,
    ensureDefaultAgent,
    setDefaultAgent,
    setPreferredModel,
    markReconnectRequired,
    markReconnected,
  };
});
