import { toastError } from '@/composables/useToast';
import { computed, onUnmounted, ref } from 'vue';

import {
  buildVec0TargetPath,
  isVec0Installed,
  resolveVec0ReleaseMapping,
} from '@/lib/vec0-detector';

export function useOnboardingVec0Download() {
  const loading = ref(true);
  const loadError = ref<string | null>(null);
  const downloadError = ref<string | null>(null);
  const downloading = ref(false);
  const status = ref<Vec0Status | null>(null);

  let disposed = false;
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  const mapping = computed(() => {
    if (!status.value) return null;
    return resolveVec0ReleaseMapping(status.value.platform, status.value.arch);
  });

  const downloadProgress = computed(() => status.value?.downloadProgress ?? null);
  const hasDownloadProgress = computed(() => (downloadProgress.value?.total ?? 0) > 0);
  const downloadPercent = computed(() => {
    const progress = downloadProgress.value;
    if (!progress || progress.total <= 0) return 0;
    return Math.max(0, Math.min(100, (progress.current / progress.total) * 100));
  });
  const isInstalled = computed(() => isVec0Installed(status.value?.fileSizeBytes));
  const needsDownload = computed(
    () => !!status.value?.supported && !isInstalled.value,
  );
  const targetPath = computed(() => {
    if (status.value?.targetPath) return status.value.targetPath;
    if (!mapping.value) return null;
    return buildVec0TargetPath(mapping.value);
  });

  function stopPolling(): void {
    if (pollHandle !== null) {
      clearInterval(pollHandle);
      pollHandle = null;
    }
  }

  async function loadStatus(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      const result = await window.electronAPI?.vec0Status?.();
      if (disposed) return;
      if (!result) {
        throw new Error('Vec0 status is unavailable in this environment.');
      }
      status.value = result;
      if (!result.downloadProgress) {
        downloading.value = false;
        stopPolling();
      }
    } catch (error) {
      if (disposed) return;
      loadError.value = error instanceof Error ? error.message : String(error);
    } finally {
      if (!disposed) {
        loading.value = false;
      }
    }
  }

  function ensurePolling(): void {
    if (pollHandle !== null) return;
    pollHandle = setInterval(() => {
      void loadStatus();
    }, 1500);
  }

  async function downloadVec0(): Promise<void> {
    downloadError.value = null;
    if (isInstalled.value) return;

    const runner = window.electronAPI?.vec0Download;
    if (!runner) {
      const message = 'Vec0 download is unavailable in this environment.';
      downloadError.value = message;
      toastError(message);
      return;
    }

    downloading.value = true;
    ensurePolling();

    try {
      const result = await runner();
      if (disposed) return;
      status.value = result;
      downloading.value = !!result.downloadProgress;
      stopPolling();
    } catch (error) {
      if (disposed) return;
      downloading.value = false;
      stopPolling();
      const message = error instanceof Error ? error.message : String(error);
      downloadError.value = message;
      toastError(`Vec0 download failed: ${message}`);
    } finally {
      if (!disposed) {
        await loadStatus();
      }
    }
  }

  onUnmounted(() => {
    disposed = true;
    stopPolling();
  });

  return {
    loading,
    loadError,
    downloadError,
    downloading,
    status,
    mapping,
    downloadProgress,
    hasDownloadProgress,
    downloadPercent,
    isInstalled,
    needsDownload,
    targetPath,
    loadStatus,
    downloadVec0,
  };
}
