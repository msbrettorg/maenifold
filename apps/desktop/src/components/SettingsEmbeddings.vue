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
    <section class="settings-section" role="region" aria-label="Embeddings">
      <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
        <div class="section-header">
          <div>
            <h3>Embeddings</h3>
            <p>Embedding models for semantic search and concept similarity.</p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            class="regenerate-button"
            :disabled="regenerating"
            @click="openRegenerateDialog"
          >
            <Loader2 v-if="regenerating" :size="14" class="spin mr-1" />
            <RefreshCw v-else :size="14" class="mr-1" />
            {{ regenerating ? 'Regenerating…' : 'Regenerate embeddings' }}
          </Button>
        </div>

        <p v-if="error" class="message error" role="alert">{{ error }}</p>

        <div v-if="loading" class="loading-text">Loading model information…</div>

        <template v-else>
          <Card size="sm" class="current-model-card">
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

          <div class="model-grid">
            <Card v-for="model in models" :key="model.id" size="sm" class="model-card">
              <CardHeader>
                <div class="model-card-header">
                  <div class="model-card-title">
                    <CardTitle class="text-sm">{{ model.name }}</CardTitle>
                    <CardDescription class="model-meta">
                      {{ model.dimensions }}d ·
                      {{ model.maxTokens }} max tokens ·
                      {{ formatSize(model.sizeBytes) }} ·
                      {{ model.license }}
                    </CardDescription>
                  </div>
                  <Badge
                    :variant="statusFor(model).variant"
                    :class="model.isActive ? 'badge-active' : ''"
                  >
                    <Check v-if="model.isActive" :size="12" class="mr-1" />
                    {{ statusFor(model).label }}
                  </Badge>
                </div>
              </CardHeader>

              <CardContent class="model-extras">
                <div v-if="model.mrlDimensions && model.mrlDimensions.length" class="mrl-row">
                  <span class="extras-label">MRL:</span>
                  <span class="extras-value">{{ model.mrlDimensions.join(' / ') }}</span>
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <button
                        type="button"
                        class="info-trigger"
                        aria-label="What is MRL?"
                      >
                        <Info :size="12" />
                      </button>
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

                <div v-if="formatMteb(model.mtebRetrieval)" class="mteb-row">
                  <span class="extras-label">MTEB retrieval:</span>
                  <span class="extras-value">{{ formatMteb(model.mtebRetrieval) }}</span>
                </div>
              </CardContent>

              <CardFooter class="model-actions">
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
                    class="spin mr-1"
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
                      class="spin mr-1"
                    />
                    {{ pending[model.id] === 'activating' ? 'Activating…' : 'Activate' }}
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    class="delete-button"
                    :disabled="!!pending[model.id]"
                    @click="openDeleteDialog(model)"
                  >
                    <Loader2
                      v-if="pending[model.id] === 'deleting'"
                      :size="14"
                      class="spin mr-1"
                    />
                    <Trash2 v-else :size="14" class="mr-1" />
                    {{ pending[model.id] === 'deleting' ? 'Deleting…' : 'Delete' }}
                  </Button>
                </template>

                <!-- Active: no destructive controls -->
                <span v-else class="active-note">
                  Active model — switch to another installed model to manage this one.
                </span>
              </CardFooter>
            </Card>
          </div>

          <div v-if="hasDimensionWarning" class="warning-banner" role="status">
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

<style scoped>
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.section-header {
  margin-bottom: 0.75rem;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  color: var(--color-text-secondary);
  display: flex;
  justify-content: space-between;
  gap: 1rem;
}

.section-header h3,
.section-header p,
.message {
  margin: 0;
}

.section-header h3 {
  font-family: var(--font-display);
  font-weight: 500;
}

.section-header p {
  color: var(--color-text-secondary);
}

.message.error {
  color: var(--color-danger);
  font-size: 0.85rem;
  margin-top: 0.5rem;
}

.loading-text {
  padding: 1rem;
  color: var(--color-text-secondary);
  font-size: 0.85rem;
}

.current-model-card {
  margin-top: 0.5rem;
}

.model-grid {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-top: 0.75rem;
}

.model-card-header {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-start;
  width: 100%;
}

.model-card-title {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.model-meta {
  font-size: 0.75rem;
}

.model-extras {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

.mrl-row,
.mteb-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.extras-label {
  color: var(--color-text-tertiary);
}

.extras-value {
  color: var(--color-text-primary);
  font-variant-numeric: tabular-nums;
}

.info-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 0;
  padding: 2px;
  color: var(--color-text-tertiary);
  cursor: help;
  border-radius: 9999px;
}

.info-trigger:hover {
  color: var(--color-text-primary);
}

.model-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  border-top: 1px solid var(--color-border-card);
  padding-top: 0.75rem;
  margin-top: 0.25rem;
}

.delete-button {
  color: var(--color-danger);
}

.delete-button:hover {
  color: var(--color-danger);
}

.regenerate-button {
  border-radius: 4px;
  flex-shrink: 0;
}

.active-note {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.badge-active {
  background-color: color-mix(in srgb, var(--color-success) 18%, transparent);
  color: var(--color-success);
  border-color: color-mix(in srgb, var(--color-success) 30%, transparent);
}

.warning-banner {
  margin-top: 0.75rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius);
  background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  color: var(--color-warning);
  font-size: 0.8rem;
}

.spin {
  animation: spin 1s linear infinite;
}

.mr-1 {
  margin-right: 0.25rem;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 640px) {
  .section-header,
  .model-card-header {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
