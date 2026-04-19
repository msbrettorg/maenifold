import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';

const STORAGE_KEY = 'mcpSettings';

type McpSettingsTab = 'servers' | 'runtime';

export interface McpSettingsState {
  activeTab: McpSettingsTab;
  selectedServerName: string;
  showOnlyErrored: boolean;
  reconnectRequired: boolean;
}

function createDefaultState(): McpSettingsState {
  return {
    activeTab: 'servers',
    selectedServerName: '',
    showOnlyErrored: false,
    reconnectRequired: false,
  };
}

function mergeState(saved: unknown): McpSettingsState {
  const defaults = createDefaultState();
  if (!saved || typeof saved !== 'object') {
    return defaults;
  }

  const value = saved as Partial<McpSettingsState>;
  return {
    activeTab: value.activeTab ?? defaults.activeTab,
    selectedServerName: value.selectedServerName ?? defaults.selectedServerName,
    showOnlyErrored: value.showOnlyErrored ?? defaults.showOnlyErrored,
    reconnectRequired: value.reconnectRequired ?? defaults.reconnectRequired,
  };
}

export const useMcpSettingsStore = defineStore('mcpSettings', () => {
  const settings = ref<McpSettingsState>(createDefaultState());
  const initialized = ref(false);

  watch(
    settings,
    (next) => {
      if (!initialized.value) {
        return;
      }
      writeStorageValue(PREFS_STORE_PATH, STORAGE_KEY, next);
    },
    { deep: true },
  );

  function init() {
    settings.value = mergeState(readStorageValue<McpSettingsState>(PREFS_STORE_PATH, STORAGE_KEY));
    initialized.value = true;
  }

  function markConfigChanged() {
    settings.value.reconnectRequired = true;
  }

  function markReconnected() {
    settings.value.reconnectRequired = false;
  }

  return {
    settings,
    init,
    markConfigChanged,
    markReconnected,
  };
});
