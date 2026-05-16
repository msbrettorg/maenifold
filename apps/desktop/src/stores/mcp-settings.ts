import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { PREFS_STORE_PATH } from '../lib/constants';
import { storageKey } from '../lib/storage-key';

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

export const useMcpSettingsStore = defineStore('mcpSettings', () => {
  const settings = useLocalStorage<McpSettingsState>(
    storageKey(PREFS_STORE_PATH, STORAGE_KEY),
    createDefaultState(),
    {
      mergeDefaults: true,
      serializer: StorageSerializers.object,
    },
  );

  function init() {}

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
