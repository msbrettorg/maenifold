import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { onUnmounted, ref } from 'vue';
import { storageKey } from '../lib/storage-key';

const STORE_NS = 'voice';
const ENABLED_KEY = 'enabled';
const POLL_INTERVAL_MS = 5000;
const LAUNCH_RECHECK_DELAY_MS = 2000;

export const useVoiceHandyStore = defineStore('voice-handy', () => {
  const installed = ref<boolean | null>(null);
  const running = ref(false);
  const path = ref<string | null>(null);
  const installHint = ref('');
  const enabled = useLocalStorage<boolean>(
    storageKey(STORE_NS, ENABLED_KEY),
    false,
    { serializer: StorageSerializers.object },
  );
  const lastError = ref<string | null>(null);

  const hasElectronApi = typeof window !== 'undefined'
    && window.electronAPI
    && typeof window.electronAPI.voiceCheck === 'function';

  let pollInterval: ReturnType<typeof setInterval> | null = null;

  async function check() {
    if (!hasElectronApi) {
      return;
    }

    try {
      const voiceCheck = window.electronAPI?.voiceCheck;
      if (!voiceCheck) {
        return;
      }

      const result = await voiceCheck();
      installed.value = result.installed;
      running.value = result.running;
      path.value = result.path;
      installHint.value = result.installHint;
      lastError.value = null;
    } catch (error: unknown) {
      lastError.value = error instanceof Error ? error.message : String(error);
    }
  }

  async function launch() {
    if (!hasElectronApi || typeof window.electronAPI?.voiceLaunch !== 'function') {
      return;
    }

    try {
      const result = await window.electronAPI.voiceLaunch();
      if (!result.ok) {
        lastError.value = result.error ?? 'Voice launch failed';
        return;
      }

      lastError.value = null;
      setTimeout(() => {
        void check();
      }, LAUNCH_RECHECK_DELAY_MS);
    } catch (error: unknown) {
      lastError.value = error instanceof Error ? error.message : String(error);
    }
  }

  function setEnabled(value: boolean) {
    enabled.value = value;
  }

  function startPolling() {
    if (!hasElectronApi || pollInterval) {
      return;
    }

    void check();
    pollInterval = setInterval(() => {
      if (typeof document !== 'undefined' && document.hasFocus()) {
        void check();
      }
    }, POLL_INTERVAL_MS);
  }

  function stopPolling() {
    if (!pollInterval) {
      return;
    }

    clearInterval(pollInterval);
    pollInterval = null;
  }

  if (hasElectronApi) {
    startPolling();
  }

  onUnmounted(() => {
    stopPolling();
  });

  return {
    installed,
    running,
    path,
    installHint,
    enabled,
    lastError,
    check,
    launch,
    setEnabled,
    startPolling,
    stopPolling,
  };
});
