import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { watch } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readLastUsedAgent, writeLastUsedAgent } from '../lib/preferred-agent';
import { storageKey } from '../lib/storage-key';

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

export const useAgentsSettingsStore = defineStore('agentsSettings', () => {
  const settings = useLocalStorage<AgentsSettingsState>(
    storageKey(PREFS_STORE_PATH, STORAGE_KEY),
    createDefaultState(),
    {
      mergeDefaults: true,
      serializer: StorageSerializers.object,
    },
  );
  const legacyModelPreferences = useLocalStorage<Record<string, string> | null>(
    storageKey(PREFS_STORE_PATH, LEGACY_MODEL_PREFERENCE_KEY),
    null,
    { serializer: StorageSerializers.object },
  );

  watch(
    settings,
    (next) => {
      legacyModelPreferences.value = next.defaults.preferredModelByAgent;
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
    if (Object.keys(settings.value.defaults.preferredModelByAgent).length === 0) {
      settings.value.defaults.preferredModelByAgent = legacyModelPreferences.value ?? {};
    }
    ensureDefaultAgent(agentNames);
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
