import { toastSuccess, toastWarning, toastError } from '@/composables/useToast';
import { computed, onUnmounted, reactive, ref } from 'vue';
import { useMcpClient } from './useMcpClient';

export interface ModelStatus {
  loaded: boolean;
  model?: {
    filename: string;
    dimensions: number;
    maxTokens: number;
  };
}

export interface ModelInfo {
  id: string;
  name: string;
  dimensions: number;
  maxTokens: number;
  sizeBytes: number;
  license: string;
  huggingfaceRepo: string;
  installed: boolean;
  isActive: boolean;
  released?: string;
  mrlDimensions?: number[];
  mtebRetrieval?: number;
  downloadProgress?: {
    current: number;
    total: number;
    filename?: string;
  } | null;
}

interface ModelListResponse {
  models: ModelInfo[];
  currentDimensions: number;
}

interface EmbeddingActionResponse {
  activated?: boolean;
  downloaded?: boolean;
  deleted?: boolean;
  regenerated?: boolean;
  modelId?: string;
  modelChanged?: boolean;
  reembedRequired?: boolean;
  vectorsCleared?: boolean;
  dimensionsDiffer?: boolean;
  message: string;
}

export type PendingAction = 'downloading' | 'activating' | 'deleting';

export function useEmbeddingsActions() {
  const { callTool, connect, connected } = useMcpClient();

  const loading = ref(true);
  const error = ref('');
  const status = ref<ModelStatus | null>(null);
  const models = ref<ModelInfo[]>([]);
  const currentDimensions = ref<number>(0);
  const pending = reactive<Record<string, PendingAction | undefined>>({});
  const regenerating = ref(false);

  const activeModel = computed(() => models.value.find((m) => m.isActive) ?? null);

  const hasDimensionWarning = computed(() => {
    if (!currentDimensions.value) return false;
    return models.value.some((m) => m.installed && m.dimensions !== currentDimensions.value);
  });

  const anyDownloading = computed(() =>
    Object.values(pending).some((s) => s === 'downloading')
  );

  let pollHandle: ReturnType<typeof setInterval> | null = null;

  async function loadData(): Promise<void> {
    try {
      if (!connected.value) {
        await connect();
      }
      const [statusResult, listResult] = await Promise.all([
        callTool<ModelStatus>('EmbeddingModels', { action: 'status' }),
        callTool<ModelListResponse>('EmbeddingModels', { action: 'list' }),
      ]);
      status.value = statusResult ?? null;
      models.value = listResult?.models ?? [];
      currentDimensions.value = listResult?.currentDimensions ?? 0;
      for (const model of models.value) {
        if (model.installed && pending[model.id] === 'downloading') {
          pending[model.id] = undefined;
        }
      }
      error.value = '';
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    }
  }

  async function initialLoad(): Promise<void> {
    loading.value = true;
    await loadData();
    loading.value = false;
  }

  function stopPolling(): void {
    if (pollHandle !== null) {
      clearInterval(pollHandle);
      pollHandle = null;
    }
  }

  function ensurePolling(): void {
    if (pollHandle !== null) return;
    pollHandle = setInterval(() => {
      if (!anyDownloading.value) {
        stopPolling();
        return;
      }
      void loadData();
    }, 3000);
  }

  async function downloadModel(model: ModelInfo): Promise<void> {
    if (pending[model.id]) return;
    pending[model.id] = 'downloading';
    error.value = '';
    try {
      const response = await callTool<EmbeddingActionResponse>('EmbeddingModels', {
        action: 'download',
        modelId: model.id,
      });
      ensurePolling();
      await loadData();
      toastSuccess(response.message ?? `Downloaded ${model.name}.`);
    } catch (e) {
      pending[model.id] = undefined;
      const message = e instanceof Error ? e.message : String(e);
      error.value = message;
      toastError(`Failed to download ${model.name}: ${message}`);
    }
  }

  async function activateModel(model: ModelInfo): Promise<void> {
    if (pending[model.id]) return;
    pending[model.id] = 'activating';
    error.value = '';
    try {
      const response = await callTool<EmbeddingActionResponse>('EmbeddingModels', {
        action: 'activate',
        modelId: model.id,
      });
      await loadData();
      const restartPrompt = response.reembedRequired
        ? ' Restart maenifold to load the new model. Vectors will rebuild on next sync.'
        : '';
      const message = `${response.message ?? `Activated ${model.name}.`}${restartPrompt}`;
      if (response.vectorsCleared === false && response.modelChanged === true) {
        toastWarning(message);
      } else {
        toastSuccess(message);
      }
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      error.value = message;
      toastError(`Failed to activate ${model.name}: ${message}`);
    } finally {
      pending[model.id] = undefined;
    }
  }

  async function deleteModel(model: ModelInfo): Promise<void> {
    if (pending[model.id]) return;
    pending[model.id] = 'deleting';
    error.value = '';
    try {
      const response = await callTool<EmbeddingActionResponse>('EmbeddingModels', {
        action: 'delete',
        modelId: model.id,
      });
      await loadData();
      toastSuccess(response.message ?? `Deleted ${model.name}.`);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      error.value = message;
      toastError(`Failed to delete ${model.name}: ${message}`);
    } finally {
      pending[model.id] = undefined;
    }
  }

  async function regenerateEmbeddings(): Promise<void> {
    if (regenerating.value) return;
    regenerating.value = true;
    error.value = '';
    try {
      const response = await callTool<EmbeddingActionResponse>('EmbeddingModels', {
        action: 'regenerate',
      });
      toastSuccess(response.message ?? 'Embeddings cleared. Run sync to rebuild.');
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      error.value = message;
      toastError(`Failed to regenerate embeddings: ${message}`);
    } finally {
      regenerating.value = false;
    }
  }

  onUnmounted(() => {
    stopPolling();
  });

  return {
    loading,
    error,
    status,
    models,
    currentDimensions,
    pending,
    regenerating,
    activeModel,
    hasDimensionWarning,
    initialLoad,
    downloadModel,
    activateModel,
    deleteModel,
    regenerateEmbeddings,
  };
}

export function formatModelSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${Math.round(bytes / 1_000_000)} MB`;
  return `${Math.round(bytes / 1_000)} KB`;
}

export function formatMtebScore(score: number | undefined): string | null {
  if (typeof score !== 'number' || Number.isNaN(score)) return null;
  return score <= 1 ? `${(score * 100).toFixed(1)}%` : score.toFixed(2);
}

export function statusForModel(
  model: ModelInfo,
): { label: string; variant: 'default' | 'secondary' | 'outline' } {
  if (model.isActive) return { label: 'Active', variant: 'default' };
  if (model.installed) return { label: 'Installed', variant: 'secondary' };
  return { label: 'Not installed', variant: 'outline' };
}
