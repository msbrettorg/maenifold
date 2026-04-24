import { computed, onUnmounted, ref, type Ref } from 'vue';

import { useMcpClient } from './useMcpClient';

export interface OnboardingModelInfo {
  id: string;
  name: string;
  dimensions: number;
  maxTokens: number;
  sizeBytes: number;
  installed: boolean;
  isActive: boolean;
  downloadProgress?: {
    current: number;
    total: number;
    filename?: string;
  } | null;
}

interface ModelListResponse {
  models: OnboardingModelInfo[];
  currentDimensions: number;
}

interface UseOnboardingModelDownloadOptions {
  advance?: () => void;
  modelSkipped?: Ref<boolean> | undefined;
  selectedModelName?: Ref<string | null> | undefined;
}

export function useOnboardingModelDownload(options: UseOnboardingModelDownloadOptions) {
  const { callTool, connect, connected } = useMcpClient();

  const loading = ref(true);
  const loadError = ref<string | null>(null);
  const downloadError = ref<string | null>(null);
  const downloading = ref(false);
  const models = ref<OnboardingModelInfo[]>([]);
  const selectedId = ref<string | null>(null);

  let disposed = false;
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  const selectedModel = computed<OnboardingModelInfo | null>(
    () => models.value.find((model) => model.id === selectedId.value) ?? null,
  );
  const selectedModelProgress = computed(() => selectedModel.value?.downloadProgress ?? null);
  const hasDownloadProgress = computed(
    () => (selectedModelProgress.value?.total ?? 0) > 0,
  );
  const downloadPercent = computed<number>(() => {
    const progress = selectedModelProgress.value;
    if (!progress || progress.total <= 0) {
      return 0;
    }
    return Math.max(0, Math.min(100, (progress.current / progress.total) * 100));
  });

  function stopPolling(): void {
    if (pollHandle !== null) {
      clearInterval(pollHandle);
      pollHandle = null;
    }
  }

  function markModelConfigured(model: OnboardingModelInfo | null): void {
    if (options.modelSkipped) options.modelSkipped.value = false;
    if (options.selectedModelName) options.selectedModelName.value = model?.name ?? null;
  }

  function finishDownload(model: OnboardingModelInfo | null): void {
    stopPolling();
    downloading.value = false;
    markModelConfigured(model);
    options.advance?.();
  }

  async function loadModels(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      if (!connected.value) {
        await connect();
      }
      if (disposed) return;
      const result = await callTool<ModelListResponse>('EmbeddingModels', { action: 'list' });
      if (disposed) return;
      models.value = result?.models ?? [];
      if (selectedId.value === null && models.value.length > 0) {
        const active = models.value.find((model) => model.isActive);
        const installed = models.value.find((model) => model.installed);
        selectedId.value = (active ?? installed ?? models.value[0]).id;
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

  async function syncDownloadState(): Promise<void> {
    await loadModels();
    if (disposed || !downloading.value) {
      return;
    }

    const latestModel = models.value.find((model) => model.id === selectedId.value) ?? null;
    if (latestModel?.installed || latestModel?.isActive) {
      finishDownload(latestModel);
    }
  }

  function ensurePolling(): void {
    if (pollHandle !== null) return;
    pollHandle = setInterval(() => {
      void syncDownloadState();
    }, 1500);
  }

  function selectModel(model: OnboardingModelInfo): void {
    if (downloading.value) return;
    selectedId.value = model.id;
  }

  async function downloadSelectedModel(): Promise<void> {
    const model = selectedModel.value;
    if (model === null) return;

    downloadError.value = null;
    if (model.installed || model.isActive) {
      markModelConfigured(model);
      options.advance?.();
      return;
    }

    downloading.value = true;
    markModelConfigured(model);

    try {
      if (!connected.value) {
        await connect();
      }
      if (disposed) return;
      await callTool('EmbeddingModels', {
        action: 'download',
        modelId: model.id,
      });
      if (disposed) return;
      await syncDownloadState();
      if (!disposed && downloading.value) {
        ensurePolling();
      }
    } catch (error) {
      if (disposed) return;
      downloading.value = false;
      stopPolling();
      downloadError.value = error instanceof Error ? error.message : String(error);
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
    models,
    selectedModel,
    selectedModelProgress,
    hasDownloadProgress,
    downloadPercent,
    loadModels,
    selectModel,
    downloadSelectedModel,
  };
}
