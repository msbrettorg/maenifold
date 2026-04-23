<!-- audit-ok: max-loc -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import { Download, Loader2, Trash2, Check, Info, RefreshCw } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { useMcpClient } from '../composables/useMcpClient';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

interface ModelStatus {
  loaded: boolean;
  model?: {
    filename: string;
    dimensions: number;
    maxTokens: number;
  };
}

interface ModelInfo {
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

type PendingAction = 'downloading' | 'activating' | 'deleting';

const { callTool, connect, connected } = useMcpClient();

const loading = ref(true);
const error = ref('');
const status = ref<ModelStatus | null>(null);
const models = ref<ModelInfo[]>([]);
const currentDimensions = ref<number>(0);

// Per-model action state, keyed by model id.
const pending = reactive<Record<string, PendingAction | undefined>>({});

// Dialog state for confirmations.
type DialogType = 'activate' | 'delete' | 'regenerate';
const dialog = ref<{ type: DialogType; modelId: string } | null>(null);

const regenerating = ref(false);

const dialogModel = computed<ModelInfo | null>(() => {
  if (!dialog.value) return null;
  return models.value.find((m) => m.id === dialog.value!.modelId) ?? null;
});

const dialogOpen = computed({
  get: () => dialog.value !== null,
  set: (open: boolean) => {
    if (!open) dialog.value = null;
  },
});

const activeModel = computed(() => models.value.find((m) => m.isActive) ?? null);

const hasDimensionWarning = computed(() => {
  if (!currentDimensions.value) return false;
  return models.value.some((m) => m.installed && m.dimensions !== currentDimensions.value);
});

const anyDownloading = computed(() =>
  Object.values(pending).some((s) => s === 'downloading')
);

let pollHandle: ReturnType<typeof setInterval> | null = null;

function formatSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${Math.round(bytes / 1_000_000)} MB`;
  return `${Math.round(bytes / 1_000)} KB`;
}

function formatMteb(score: number | undefined): string | null {
  if (typeof score !== 'number' || Number.isNaN(score)) return null;
  // Display as percentage if 0–1, otherwise as-is.
  return score <= 1 ? `${(score * 100).toFixed(1)}%` : score.toFixed(2);
}

function statusFor(model: ModelInfo): { label: string; variant: 'default' | 'secondary' | 'outline' } {
  if (model.isActive) return { label: 'Active', variant: 'default' };
  if (model.installed) return { label: 'Installed', variant: 'secondary' };
  return { label: 'Not installed', variant: 'outline' };
}

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

    // Clear any "downloading" flags for models that are now installed.
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

function stopPolling(): void {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}

async function downloadModel(model: ModelInfo): Promise<void> {
  if (pending[model.id]) return;
  pending[model.id] = 'downloading';
  error.value = '';
  try {
    await callTool('EmbeddingModels', { action: 'download', modelId: model.id });
    // Begin polling until installed.
    ensurePolling();
    // Kick a refresh shortly after to catch fast completions.
    await loadData();
  } catch (e) {
    pending[model.id] = undefined;
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function openActivateDialog(model: ModelInfo): void {
  dialog.value = { type: 'activate', modelId: model.id };
}

function openDeleteDialog(model: ModelInfo): void {
  dialog.value = { type: 'delete', modelId: model.id };
}

function openRegenerateDialog(): void {
  dialog.value = { type: 'regenerate', modelId: '' };
}

async function regenerateEmbeddings(): Promise<void> {
  if (regenerating.value) return;
  regenerating.value = true;
  error.value = '';
  try {
    await callTool('EmbeddingModels', { action: 'regenerate' });
    toast.success('Embeddings cleared. Run sync to rebuild.');
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    error.value = message;
    toast.error(`Failed to regenerate embeddings: ${message}`);
  } finally {
    regenerating.value = false;
  }
}

async function confirmDialog(): Promise<void> {
  const current = dialog.value;
  if (!current) return;
  dialog.value = null;

  if (current.type === 'regenerate') {
    await regenerateEmbeddings();
    return;
  }

  const model = models.value.find((m) => m.id === current.modelId);
  if (!model) return;

  if (current.type === 'activate') {
    await activateModel(model);
  } else {
    await deleteModel(model);
  }
}

async function activateModel(model: ModelInfo): Promise<void> {
  if (pending[model.id]) return;
  pending[model.id] = 'activating';
  error.value = '';
  try {
    await callTool('EmbeddingModels', { action: 'activate', modelId: model.id });
    await loadData();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    pending[model.id] = undefined;
  }
}

async function deleteModel(model: ModelInfo): Promise<void> {
  if (pending[model.id]) return;
  pending[model.id] = 'deleting';
  error.value = '';
  try {
    await callTool('EmbeddingModels', { action: 'delete', modelId: model.id });
    await loadData();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    pending[model.id] = undefined;
  }
}

onMounted(() => {
  void initialLoad();
});

onUnmounted(() => {
  stopPolling();
});
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <section class="flex flex-col gap-4" role="region" aria-label="Embeddings">
      <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
        <div class="flex justify-between gap-4 mb-3 text-[11px] font-semibold tracking-[0.06em] text-text-secondary max-sm:flex-col max-sm:items-stretch">
          <div>
            <h3 class="m-0 font-display font-medium">Embeddings</h3>
            <p class="m-0 text-text-secondary">Embedding models for semantic search and concept similarity.</p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            class="rounded shrink-0"
            :disabled="regenerating"
            @click="openRegenerateDialog"
          >
            <Loader2 v-if="regenerating" :size="14" class="animate-spin mr-1" />
            <RefreshCw v-else :size="14" class="mr-1" />
            {{ regenerating ? 'Regenerating…' : 'Regenerate embeddings' }}
          </Button>
        </div>

        <p v-if="error" class="m-0 text-danger text-[0.85rem] mt-2" role="alert">{{ error }}</p>

        <div v-if="loading" class="p-4 text-text-secondary text-[0.85rem]">Loading model information…</div>

        <template v-else>
          <Card size="sm" class="mt-2">
            <CardHeader>
              <CardTitle class="text-sm">Current model</CardTitle>
              <CardDescription>
                <template v-if="activeModel">
                  {{ activeModel.name }} — {{ activeModel.dimensions }}d,
                  {{ activeModel.maxTokens }} max tokens
                </template>
                <template v-else-if="status?.loaded">
                  {{ status.model?.filename }} — {{ status.model?.dimensions }}d,
                  {{ status.model?.maxTokens }} max tokens
                </template>
                <template v-else>No model loaded</template>
              </CardDescription>
            </CardHeader>
          </Card>

          <div class="flex flex-col gap-3 mt-3">
            <Card v-for="model in models" :key="model.id" size="sm">
              <CardHeader>
                <div class="flex justify-between gap-4 items-start w-full max-sm:flex-col max-sm:items-stretch">
                  <div class="flex flex-col gap-1 min-w-0">
                    <CardTitle class="text-sm">{{ model.name }}</CardTitle>
                    <CardDescription class="text-[0.75rem]">
                      {{ model.dimensions }}d ·
                      {{ model.maxTokens }} max tokens ·
                      {{ formatSize(model.sizeBytes) }} ·
                      {{ model.license }}
                    </CardDescription>
                  </div>
                  <!-- audit-ok: dynamic-binding -->
                  <Badge
                    :variant="statusFor(model).variant"
                    :style="model.isActive ? { backgroundColor: 'color-mix(in srgb, var(--color-success) 18%, transparent)', color: 'var(--color-success)', borderColor: 'color-mix(in srgb, var(--color-success) 30%, transparent)' } : undefined"
                  >
                    <Check v-if="model.isActive" :size="12" class="mr-1" />
                    {{ statusFor(model).label }}
                  </Badge>
                </div>
              </CardHeader>

              <CardContent class="flex flex-col gap-1 text-[0.75rem] text-text-secondary">
                <div v-if="model.mrlDimensions && model.mrlDimensions.length" class="flex items-center gap-[0.4rem]">
                  <span class="text-text-tertiary">MRL:</span>
                  <span class="text-text-primary tabular-nums">{{ model.mrlDimensions.join(' / ') }}</span>
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <Button
                        type="button"
                        class="inline-flex items-center justify-center bg-transparent border-0 p-[2px] text-text-tertiary cursor-help rounded-full hover:text-text-primary"
                        aria-label="What is MRL?"
                      >
                        <Info :size="12" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent
                      side="top"
                      class="max-w-[260px] rounded-[4px] bg-[var(--surface-highest)] text-[var(--color-text-primary)]"
                    >
                      Matryoshka Representation Learning — embeddings can be
                      truncated to these smaller dimensions while keeping most
                      retrieval quality.
                    </TooltipContent>
                  </Tooltip>
                </div>

                <div v-if="formatMteb(model.mtebRetrieval)" class="flex items-center gap-[0.4rem]">
                  <span class="text-text-tertiary">MTEB retrieval:</span>
                  <span class="text-text-primary tabular-nums">{{ formatMteb(model.mtebRetrieval) }}</span>
                </div>
              </CardContent>

              <CardFooter class="flex flex-wrap gap-2 items-center border-t border-border-card pt-3 mt-1">
                <!-- Not installed: Download -->
                <Button
                  v-if="!model.installed"
                  type="button"
                  variant="default"
                  size="sm"
                  :disabled="pending[model.id] === 'downloading'"
                  @click="downloadModel(model)"
                >
                  <Loader2
                    v-if="pending[model.id] === 'downloading'"
                    :size="14"
                    class="animate-spin mr-1"
                  />
                  <Download v-else :size="14" class="mr-1" />
                  {{
                    pending[model.id] === 'downloading'
                      ? (model.downloadProgress && model.downloadProgress.total > 0
                          ? `Downloading… ${formatSize(model.downloadProgress.current)} / ${formatSize(model.downloadProgress.total)}`
                          : 'Downloading…')
                      : 'Download'
                  }}
                </Button>

                <!-- Installed but not active: Activate + Delete -->
                <template v-else-if="!model.isActive">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    :disabled="!!pending[model.id]"
                    @click="openActivateDialog(model)"
                  >
                    <Loader2
                      v-if="pending[model.id] === 'activating'"
                      :size="14"
                      class="animate-spin mr-1"
                    />
                    {{ pending[model.id] === 'activating' ? 'Activating…' : 'Activate' }}
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    class="text-danger hover:text-danger"
                    :disabled="!!pending[model.id]"
                    @click="openDeleteDialog(model)"
                  >
                    <Loader2
                      v-if="pending[model.id] === 'deleting'"
                      :size="14"
                      class="animate-spin mr-1"
                    />
                    <Trash2 v-else :size="14" class="mr-1" />
                    {{ pending[model.id] === 'deleting' ? 'Deleting…' : 'Delete' }}
                  </Button>
                </template>

                <!-- Active: no destructive controls -->
                <span v-else class="text-[0.75rem] text-text-tertiary">
                  Active model — switch to another installed model to manage this one.
                </span>
              </CardFooter>
            </Card>
          </div>

          <!-- audit-ok: dynamic-binding -->
          <div v-if="hasDimensionWarning" :style="{ background: 'color-mix(in srgb, var(--color-warning) 12%, transparent)', color: 'var(--color-warning)' }" class="mt-3 px-4 py-3 rounded-[var(--radius)] text-[0.8rem]" role="status">
            Switching to a model with different dimensions requires regenerating
            all embeddings.
          </div>
        </template>
      </div>

      <AlertDialog v-model:open="dialogOpen">
        <AlertDialogContent class="sm:max-w-[28rem]">
          <AlertDialogHeader>
            <AlertDialogTitle>
              <template v-if="dialog?.type === 'activate'">
                Activate {{ dialogModel?.name ?? 'model' }}?
              </template>
              <template v-else-if="dialog?.type === 'regenerate'">
                Regenerate all embeddings?
              </template>
              <template v-else>
                Delete {{ dialogModel?.name ?? 'model' }}?
              </template>
            </AlertDialogTitle>
            <AlertDialogDescription>
              <template v-if="dialog?.type === 'activate'">
                This will rebuild all vector embeddings. Search will be
                temporarily unavailable while the index is regenerated.
              </template>
              <template v-else-if="dialog?.type === 'regenerate'">
                This will delete all vector embeddings and rebuild them from
                scratch. Search will be unavailable until the rebuild
                completes.
              </template>
              <template v-else>
                This will remove the downloaded model files from disk. You can
                download the model again later if you change your mind.
              </template>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction @click="confirmDialog">
              <template v-if="dialog?.type === 'activate'">Activate</template>
              <template v-else-if="dialog?.type === 'regenerate'">Regenerate</template>
              <template v-else>Delete</template>
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  </TooltipProvider>
</template>

