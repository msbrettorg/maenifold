<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import {
  Check,
  Download,
  Package,
  Play,
  RefreshCw,
  Trash2,
  X,
} from '@lucide/vue';

import { useEmbeddingsStore, type EmbeddingModel } from '../stores/embeddings';

type MutationKind = 'download' | 'activate' | 'delete' | 'regenerate';
type MutationTone = 'primary' | 'danger' | 'warn';

interface PendingMutation {
  kind: MutationKind;
  title: string;
  summary: string;
  confirmLabel: string;
  tone: MutationTone;
  parameters: Record<string, unknown>;
  details: Array<{ label: string; value: string }>;
  warning: string | null;
}

const embeddingsStore = useEmbeddingsStore();

const pendingMutation = ref<PendingMutation | null>(null);
const mutationError = ref<string | null>(null);
const mutationPending = ref(false);

const activeCatalogModel = computed<EmbeddingModel | null>(() => {
  if (embeddingsStore.status?.activeModelId) {
    const matching = embeddingsStore.catalog.find(
      (model) => model.id === embeddingsStore.status?.activeModelId
    );
    if (matching) return matching;
  }

  return embeddingsStore.catalog.find((model) => model.isActive) ?? null;
});

const activeModelIdLabel = computed<string>(
  () => embeddingsStore.status?.activeModelId ?? activeCatalogModel.value?.id ?? 'Not configured'
);

const activeModelDetails = computed<string>(() => {
  const detailParts: string[] = [];
  const runtimeModel = embeddingsStore.status?.model;
  const catalogModel = activeCatalogModel.value;

  if (runtimeModel?.dimensions ?? catalogModel?.dimensions) {
    detailParts.push(`${runtimeModel?.dimensions ?? catalogModel?.dimensions} dims`);
  }

  if (runtimeModel?.maxTokens ?? catalogModel?.maxTokens) {
    detailParts.push(
      `${(runtimeModel?.maxTokens ?? catalogModel?.maxTokens)?.toLocaleString()} max tokens`
    );
  }

  if (runtimeModel?.filename) {
    detailParts.push(runtimeModel.filename);
  } else if (catalogModel?.installedLayout) {
    detailParts.push(`${catalogModel.installedLayout} layout`);
  }

  return detailParts.join(' · ') || 'Waiting for daemon status…';
});

const activeModelMessage = computed<string>(() => {
  return (
    embeddingsStore.status?.vec0.message ??
    embeddingsStore.status?.activeModelStatus.message ??
    'Polling EmbeddingModels status and catalog…'
  );
});

const healthFlags = computed<Array<{ label: string; healthy: boolean }>>(() => {
  const vec0 = embeddingsStore.status?.vec0;
  if (!vec0) {
    return [];
  }

  return [
    { label: 'installed', healthy: vec0.installed },
    { label: 'rustLoadable', healthy: vec0.rustLoadable },
    { label: 'vectorTablesReady', healthy: vec0.vectorTablesReady },
    { label: 'embeddingsPopulated', healthy: vec0.embeddingsPopulated },
  ];
});

const tableCounts = computed<Array<{ label: string; value: number | null }>>(() => {
  const tables = embeddingsStore.status?.vec0.tables;
  if (!tables) {
    return [];
  }

  return [
    { label: 'vecConcepts', value: tables.vecConcepts.count },
    { label: 'vecMemoryFiles', value: tables.vecMemoryFiles.count },
  ];
});

const sortedCatalog = computed<EmbeddingModel[]>(() => {
  return [...embeddingsStore.catalog].sort((left, right) => {
    if (left.isActive !== right.isActive) {
      return left.isActive ? -1 : 1;
    }

    if (left.installed !== right.installed) {
      return left.installed ? -1 : 1;
    }

    return 0;
  });
});

const hasCachedData = computed<boolean>(() => {
  return embeddingsStore.status !== null || embeddingsStore.catalog.length > 0;
});

const daemonUnavailable = computed<boolean>(() => isDaemonError(embeddingsStore.error));

const daemonBannerMessage = computed<string | null>(() => {
  if (!daemonUnavailable.value || !embeddingsStore.error) {
    return null;
  }

  return hasCachedData.value
    ? `Daemon unreachable. Showing last-known embeddings data. ${embeddingsStore.error}`
    : `Daemon unreachable. No cached embeddings data is available yet. ${embeddingsStore.error}`;
});

const panelError = computed<string | null>(() => {
  if (!embeddingsStore.error || daemonUnavailable.value) {
    return null;
  }

  return embeddingsStore.error;
});

function isDaemonError(message: string | null): boolean {
  if (!message) return false;

  const lower = message.toLowerCase();
  return (
    lower.includes('mcp request failed') ||
    lower.includes('failed to fetch') ||
    lower.includes('econnrefused') ||
    lower.includes('networkerror') ||
    lower.includes('mcp http')
  );
}

function isRestrictedLicense(license: string): boolean {
  const normalized = license.trim().toLowerCase();
  return !['mit', 'apache-2.0', 'apache 2.0', 'apache2'].includes(normalized);
}

function formatBytes(bytes: number | null | undefined): string {
  if (typeof bytes !== 'number' || Number.isNaN(bytes) || bytes < 0) {
    return '—';
  }

  if (bytes === 0) {
    return '0 B';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const digits = value >= 100 || unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unitIndex]}`;
}

function formatScore(score: number | null): string {
  return typeof score === 'number' ? score.toFixed(1) : '—';
}

function formatCount(value: number | null): string {
  return typeof value === 'number' ? value.toLocaleString() : '—';
}

function percentFromProgress(model: EmbeddingModel): number | null {
  const current = model.downloadProgress?.current;
  const total = model.downloadProgress?.total;

  if (typeof current !== 'number' || typeof total !== 'number' || total <= 0) {
    return null;
  }

  return Math.min(100, Math.max(0, (current / total) * 100));
}

function statusLabel(model: EmbeddingModel): string {
  if (model.isActive) return 'active';
  if (model.downloadProgress) return 'downloading';
  if (model.installed) return 'installed';
  return 'not installed';
}

function commandPreview(parameters: Record<string, unknown>): string {
  return `GatewayExecute domain="maenifold.system" command="EmbeddingModels" parameters=${JSON.stringify(
    parameters
  )}`;
}

function openMutationDialog(mutation: PendingMutation): void {
  mutationError.value = null;
  pendingMutation.value = mutation;
}

function closeMutationDialog(): void {
  if (mutationPending.value) {
    return;
  }

  pendingMutation.value = null;
  mutationError.value = null;
}

function promptDownload(model: EmbeddingModel): void {
  openMutationDialog({
    kind: 'download',
    title: `Download ${model.name}`,
    summary: `Download ${model.id} into the managed embeddings catalog.`,
    confirmLabel: 'Download model',
    tone: 'primary',
    parameters: {
      action: 'download',
      modelId: model.id,
    },
    details: [
      { label: 'Model ID', value: model.id },
      { label: 'Dimensions', value: formatCount(model.dimensions) },
      { label: 'Max tokens', value: formatCount(model.maxTokens) },
      { label: 'Size', value: formatBytes(model.sizeBytes) },
      { label: 'License', value: model.license },
    ],
    warning: isRestrictedLicense(model.license)
      ? 'This model uses a restricted license. The backend may require an allowRestrictedLicense opt-in before the command succeeds.'
      : null,
  });
}

function promptActivate(model: EmbeddingModel): void {
  openMutationDialog({
    kind: 'activate',
    title: `Activate ${model.name}`,
    summary: `Set ${model.id} as the active embedding model for semantic search.`,
    confirmLabel: 'Activate model',
    tone: 'primary',
    parameters: {
      action: 'activate',
      modelId: model.id,
    },
    details: [
      { label: 'Model ID', value: model.id },
      { label: 'Installed layout', value: model.installedLayout ?? 'unknown' },
      { label: 'Current active model', value: activeModelIdLabel.value },
      { label: 'Dimensions', value: formatCount(model.dimensions) },
    ],
    warning: isRestrictedLicense(model.license)
      ? 'Restricted-license activations require backend opt-in if the daemon enforces license confirmation.'
      : null,
  });
}

function promptDelete(model: EmbeddingModel): void {
  openMutationDialog({
    kind: 'delete',
    title: `Delete ${model.name}`,
    summary: `Remove ${model.id} from the managed embeddings catalog.`,
    confirmLabel: 'Delete model',
    tone: 'danger',
    parameters: {
      action: 'delete',
      modelId: model.id,
    },
    details: [
      { label: 'Model ID', value: model.id },
      { label: 'Installed layout', value: model.installedLayout ?? 'unknown' },
      { label: 'License', value: model.license },
      { label: 'Size', value: formatBytes(model.sizeBytes) },
    ],
    warning: 'Deleting a model removes the local ONNX asset. The active model cannot be deleted.',
  });
}

function promptRegenerate(sourceModel: EmbeddingModel | null = activeCatalogModel.value): void {
  openMutationDialog({
    kind: 'regenerate',
    title: 'Regenerate embeddings',
    summary:
      'Clear embedding metadata and immediately rebuild vectors against the active model. This fires EmbeddingModels regenerate followed by graph Sync in one operation.',
    confirmLabel: 'Regenerate now',
    tone: 'warn',
    parameters: {
      action: 'regenerate',
    },
    details: [
      { label: 'Active model', value: sourceModel?.id ?? activeModelIdLabel.value },
      {
        label: 'vecConcepts count',
        value: formatCount(embeddingsStore.status?.vec0.tables?.vecConcepts.count ?? null),
      },
      {
        label: 'vecMemoryFiles count',
        value: formatCount(embeddingsStore.status?.vec0.tables?.vecMemoryFiles.count ?? null),
      },
    ],
    warning:
      'Sync runs synchronously and can take several seconds for large memory stores. The vector tables will repopulate before this dialog closes.',
  });
}

async function submitMutation(): Promise<void> {
  if (!pendingMutation.value) {
    return;
  }

  mutationPending.value = true;
  mutationError.value = null;

  try {
    switch (pendingMutation.value.kind) {
      case 'download':
        await embeddingsStore.download(String(pendingMutation.value.parameters.modelId));
        break;
      case 'activate':
        await embeddingsStore.activate(String(pendingMutation.value.parameters.modelId));
        break;
      case 'delete':
        await embeddingsStore.delete(String(pendingMutation.value.parameters.modelId));
        break;
      case 'regenerate':
        await embeddingsStore.regenerate();
        break;
    }

    pendingMutation.value = null;
  } catch (error) {
    mutationError.value = error instanceof Error ? error.message : String(error);
  } finally {
    mutationPending.value = false;
  }
}

function rowHasDownloadAction(model: EmbeddingModel): boolean {
  return !model.installed && !model.downloadProgress;
}

function rowHasActivateAction(model: EmbeddingModel): boolean {
  return model.installed && !model.isActive;
}

function rowHasDeleteAction(model: EmbeddingModel): boolean {
  return model.installed && !model.isActive;
}

function rowHasRegenerateAction(model: EmbeddingModel): boolean {
  return model.isActive;
}

onMounted(() => {
  void embeddingsStore.refresh().catch(() => undefined);
  embeddingsStore.startPolling();
});

onUnmounted(() => {
  embeddingsStore.stopPolling();
});
</script>

<template>
  <div class="settings-embeddings">
    <div v-if="daemonBannerMessage" class="panel-banner" role="status">
      <X class="banner-icon" :size="16" :stroke-width="2.1" />
      <span>{{ daemonBannerMessage }}</span>
    </div>

    <div v-if="panelError" class="panel-error" role="alert">
      {{ panelError }}
    </div>

    <section class="card">
      <div class="card-head">
        <div class="card-title-wrap">
          <div class="card-title-row">
            <Package class="card-icon" :size="16" :stroke-width="2" />
            <h3 class="card-title">Active model: <span class="mono">{{ activeModelIdLabel }}</span></h3>
          </div>
          <p class="card-subtitle">{{ activeModelDetails }}</p>
        </div>
        <span class="runtime-pill" :class="{ 'runtime-pill--active': embeddingsStore.status?.loaded }">
          {{ embeddingsStore.status?.loaded ? 'Daemon loaded' : 'Daemon pending' }}
        </span>
      </div>

      <div class="summary-band">
        <div class="summary-copy">
          <p class="summary-text">{{ activeModelMessage }}</p>
        </div>

        <div class="health-group">
          <span
            v-for="flag in healthFlags"
            :key="flag.label"
            class="health-pill"
            :class="{ 'health-pill--ok': flag.healthy, 'health-pill--bad': !flag.healthy }"
          >
            <component :is="flag.healthy ? Check : X" :size="12" :stroke-width="2.4" />
            {{ flag.label }}
          </span>
          <span v-for="entry in tableCounts" :key="entry.label" class="health-pill health-pill--count">
            {{ entry.label }} {{ formatCount(entry.value) }}
          </span>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <div class="card-title-wrap">
          <div class="card-title-row">
            <Package class="card-icon" :size="16" :stroke-width="2" />
            <h3 class="card-title">Catalog</h3>
          </div>
          <p class="card-subtitle">
            Live model catalog from <code class="mono">EmbeddingModels list / status</code>.
          </p>
        </div>

        <button
          class="action-button action-button--warn"
          type="button"
          :disabled="mutationPending || embeddingsStore.loading"
          @click="promptRegenerate()"
        >
          <RefreshCw
            class="button-icon"
            :class="{ spinning: mutationPending && pendingMutation?.kind === 'regenerate' }"
            :size="16"
            :stroke-width="2"
          />
          Regenerate embeddings
        </button>
      </div>

      <div class="table-wrap">
        <table class="catalog-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Name</th>
              <th>Dimensions</th>
              <th>Max tokens</th>
              <th>Size</th>
              <th>License</th>
              <th>MTEB retrieval</th>
              <th>Status</th>
              <th class="actions-header">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="sortedCatalog.length === 0">
              <td colspan="9" class="empty-row">
                {{
                  embeddingsStore.loading
                    ? 'Loading embedding catalog…'
                    : 'No embedding models were returned by the daemon.'
                }}
              </td>
            </tr>
            <tr
              v-for="model in sortedCatalog"
              :key="model.id"
              :class="{ 'catalog-row--active': model.isActive }"
            >
              <td class="mono">{{ model.id }}</td>
              <td>{{ model.name }}</td>
              <td class="mono">{{ formatCount(model.dimensions) }}</td>
              <td class="mono">{{ formatCount(model.maxTokens) }}</td>
              <td class="mono">{{ formatBytes(model.sizeBytes) }}</td>
              <td>
                <span
                  class="license-pill"
                  :class="{ 'license-pill--restricted': isRestrictedLicense(model.license) }"
                >
                  {{ model.license }}
                </span>
              </td>
              <td class="mono">{{ formatScore(model.mtebRetrieval) }}</td>
              <td>
                <div class="status-cell">
                  <span
                    class="status-pill"
                    :class="`status-pill--${statusLabel(model).replace(/\s+/g, '-')}`"
                  >
                    {{ statusLabel(model) }}
                  </span>

                  <div v-if="model.downloadProgress" class="progress-block">
                    <div class="progress-bar" aria-hidden="true">
                      <div
                        class="progress-fill"
                        :style="{ width: `${percentFromProgress(model) ?? 0}%` }"
                      />
                    </div>
                    <div class="progress-text">
                      {{ formatBytes(model.downloadProgress.current) }} /
                      {{ formatBytes(model.downloadProgress.total) }}
                      <span v-if="percentFromProgress(model) !== null">
                        · {{ Math.round(percentFromProgress(model) ?? 0) }}%
                      </span>
                    </div>
                  </div>
                </div>
              </td>
              <td class="actions-cell">
                <div class="row-actions">
                  <button
                    v-if="rowHasDownloadAction(model)"
                    class="icon-button icon-button--primary"
                    type="button"
                    :disabled="mutationPending || embeddingsStore.loading"
                    :aria-label="`Download ${model.id}`"
                    :title="`Download ${model.id}`"
                    @click="promptDownload(model)"
                  >
                    <Download :size="15" :stroke-width="2.1" />
                  </button>

                  <button
                    v-else-if="model.downloadProgress"
                    class="icon-button"
                    type="button"
                    disabled
                    aria-label="Download in progress"
                    title="Download in progress"
                  >
                    <RefreshCw class="spinning" :size="15" :stroke-width="2.1" />
                  </button>

                  <button
                    v-if="rowHasActivateAction(model)"
                    class="icon-button icon-button--primary"
                    type="button"
                    :disabled="mutationPending || embeddingsStore.loading"
                    :aria-label="`Activate ${model.id}`"
                    :title="`Activate ${model.id}`"
                    @click="promptActivate(model)"
                  >
                    <Play :size="15" :stroke-width="2.1" />
                  </button>

                  <button
                    v-if="rowHasRegenerateAction(model)"
                    class="icon-button icon-button--warn"
                    type="button"
                    :disabled="mutationPending || embeddingsStore.loading"
                    :aria-label="`Regenerate embeddings for ${model.id}`"
                    :title="`Regenerate embeddings for ${model.id}`"
                    @click="promptRegenerate(model)"
                  >
                    <RefreshCw :size="15" :stroke-width="2.1" />
                  </button>

                  <button
                    v-if="rowHasDeleteAction(model)"
                    class="icon-button icon-button--danger"
                    type="button"
                    :disabled="mutationPending || embeddingsStore.loading"
                    :aria-label="`Delete ${model.id}`"
                    :title="`Delete ${model.id}`"
                    @click="promptDelete(model)"
                  >
                    <Trash2 :size="15" :stroke-width="2.1" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <div
      v-if="pendingMutation"
      class="confirm-overlay"
      @click.self="closeMutationDialog"
    >
      <div class="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="embeddings-confirm-title">
        <div class="confirm-head">
          <div>
            <h4 id="embeddings-confirm-title">{{ pendingMutation.title }}</h4>
            <p>{{ pendingMutation.summary }}</p>
          </div>
          <button
            class="icon-button"
            type="button"
            :disabled="mutationPending"
            aria-label="Close confirmation dialog"
            @click="closeMutationDialog"
          >
            <X :size="15" :stroke-width="2.1" />
          </button>
        </div>

        <div class="confirm-body">
          <dl class="details-list">
            <div v-for="detail in pendingMutation.details" :key="detail.label" class="details-row">
              <dt>{{ detail.label }}</dt>
              <dd>{{ detail.value }}</dd>
            </div>
          </dl>

          <div class="preview-block">
            <span class="preview-label">Exact MCP command</span>
            <pre>{{ commandPreview(pendingMutation.parameters) }}</pre>
          </div>

          <p v-if="pendingMutation.warning" class="confirm-warning">
            {{ pendingMutation.warning }}
          </p>

          <p v-if="mutationError" class="confirm-error" role="alert">
            {{ mutationError }}
          </p>
        </div>

        <div class="confirm-actions">
          <button
            class="secondary-button"
            type="button"
            :disabled="mutationPending"
            @click="closeMutationDialog"
          >
            Cancel
          </button>
          <button
            class="action-button"
            :class="`action-button--${pendingMutation.tone}`"
            type="button"
            :disabled="mutationPending"
            @click="submitMutation"
          >
            <RefreshCw v-if="mutationPending" class="button-icon spinning" :size="16" :stroke-width="2" />
            <span>{{ pendingMutation.confirmLabel }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-embeddings {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.15rem;
}

.panel-banner,
.panel-error,
.card {
  border-radius: 14px;
}

.panel-banner,
.panel-error {
  display: flex;
  align-items: flex-start;
  gap: 0.7rem;
  padding: 0.9rem 1rem;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-warning) 55%, var(--bg-main));
  color: var(--text-primary);
}

.panel-error {
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
  border-color: color-mix(in srgb, var(--bg-danger) 35%, var(--border-color));
}

.banner-icon {
  flex-shrink: 0;
  margin-top: 0.05rem;
  color: color-mix(in srgb, var(--bg-danger) 72%, var(--text-primary));
}

.card {
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 34%, var(--bg-main));
  overflow: hidden;
}

.card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1rem 0.85rem;
}

.card-title-wrap {
  min-width: 0;
}

.card-title-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.card-icon {
  color: var(--text-accent);
  flex-shrink: 0;
}

.card-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.card-subtitle {
  margin: 0.3rem 0 0;
  color: var(--text-secondary);
  font-size: 0.87rem;
  line-height: 1.5;
}

.runtime-pill,
.health-pill,
.status-pill,
.license-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.runtime-pill {
  flex-shrink: 0;
  padding: 0.35rem 0.7rem;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  background: var(--bg-main);
}

.runtime-pill--active {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 9%, var(--bg-main));
}

.summary-band {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.9rem;
  padding: 0 1rem 1rem;
}

.summary-copy {
  min-width: 220px;
  flex: 1 1 280px;
}

.summary-text {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.55;
}

.health-group {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  justify-content: flex-end;
}

.health-pill {
  padding: 0.3rem 0.6rem;
  border: 1px solid var(--border-color);
  background: var(--bg-main);
  color: var(--text-secondary);
}

.health-pill--ok {
  color: color-mix(in srgb, var(--bg-success) 82%, var(--text-primary));
  border-color: color-mix(in srgb, var(--bg-success) 36%, var(--border-color));
  background: color-mix(in srgb, var(--bg-success) 10%, var(--bg-main));
}

.health-pill--bad {
  color: color-mix(in srgb, var(--bg-danger) 82%, var(--text-primary));
  border-color: color-mix(in srgb, var(--bg-danger) 30%, var(--border-color));
  background: color-mix(in srgb, var(--bg-danger) 9%, var(--bg-main));
}

.health-pill--count {
  color: var(--text-primary);
}

.action-button,
.secondary-button,
.icon-button {
  border: 1px solid var(--border-color);
  background: var(--bg-main);
  color: var(--text-primary);
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease,
    color 0.15s ease,
    opacity 0.15s ease;
}

.action-button:disabled,
.secondary-button:disabled,
.icon-button:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.action-button {
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.6rem 0.85rem;
  border-radius: 10px;
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  white-space: nowrap;
}

.action-button--primary:hover:not(:disabled),
.icon-button--primary:hover:not(:disabled) {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 8%, var(--bg-main));
}

.action-button--danger,
.icon-button--danger {
  color: color-mix(in srgb, var(--bg-danger) 78%, var(--text-primary));
  border-color: color-mix(in srgb, var(--bg-danger) 24%, var(--border-color));
}

.action-button--danger:hover:not(:disabled),
.icon-button--danger:hover:not(:disabled) {
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
}

.action-button--warn,
.icon-button--warn {
  color: color-mix(in srgb, var(--text-accent) 65%, var(--bg-danger));
  border-color: color-mix(in srgb, var(--text-accent) 24%, var(--border-color));
}

.action-button--warn:hover:not(:disabled),
.icon-button--warn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--bg-warning) 42%, var(--bg-main));
}

.secondary-button {
  padding: 0.6rem 0.85rem;
  border-radius: 10px;
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
}

.secondary-button:hover:not(:disabled),
.icon-button:hover:not(:disabled) {
  background: var(--bg-hover);
}

.button-icon,
.spinning {
  flex-shrink: 0;
}

.spinning {
  animation: rotate-icon 0.8s linear infinite;
}

.table-wrap {
  overflow-x: auto;
}

.catalog-table {
  width: 100%;
  min-width: 980px;
  border-collapse: collapse;
}

.catalog-table th,
.catalog-table td {
  padding: 0.85rem 1rem;
  border-top: 1px solid var(--border-color);
  vertical-align: top;
  text-align: left;
}

.catalog-table th {
  font-size: 0.74rem;
  font-weight: 700;
  color: var(--text-muted);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.catalog-row--active {
  background: color-mix(in srgb, var(--text-accent) 5%, transparent);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
}

.license-pill,
.status-pill {
  padding: 0.25rem 0.55rem;
  border: 1px solid var(--border-color);
  background: var(--bg-main);
  color: var(--text-secondary);
}

.license-pill--restricted {
  color: color-mix(in srgb, var(--bg-danger) 70%, var(--text-primary));
  border-color: color-mix(in srgb, var(--bg-danger) 28%, var(--border-color));
  background: color-mix(in srgb, var(--bg-warning) 45%, var(--bg-main));
}

.status-cell {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.status-pill--active {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 9%, var(--bg-main));
}

.status-pill--installed {
  color: color-mix(in srgb, var(--bg-success) 85%, var(--text-primary));
  border-color: color-mix(in srgb, var(--bg-success) 35%, var(--border-color));
  background: color-mix(in srgb, var(--bg-success) 10%, var(--bg-main));
}

.status-pill--downloading {
  color: color-mix(in srgb, var(--text-accent) 70%, var(--text-primary));
  border-color: color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 8%, var(--bg-main));
}

.status-pill--not-installed {
  color: var(--text-secondary);
}

.progress-block {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.progress-bar {
  width: 100%;
  height: 8px;
  border-radius: 999px;
  overflow: hidden;
  background: color-mix(in srgb, var(--bg-sidebar) 85%, var(--bg-main));
  border: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
}

.progress-fill {
  height: 100%;
  background: linear-gradient(
    90deg,
    color-mix(in srgb, var(--text-accent) 70%, var(--bg-main)),
    var(--text-accent)
  );
}

.progress-text {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.actions-header,
.actions-cell {
  text-align: right;
}

.row-actions {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.4rem;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 9px;
}

.empty-row {
  color: var(--text-secondary);
}

.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: rgba(0, 0, 0, 0.45);
}

.confirm-dialog {
  width: min(560px, 100%);
  max-height: min(90vh, 760px);
  overflow: auto;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--bg-main);
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.3);
}

.confirm-head,
.confirm-actions {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.1rem;
}

.confirm-head {
  border-bottom: 1px solid var(--border-color);
}

.confirm-head h4 {
  margin: 0;
  font-size: 1rem;
  color: var(--text-primary);
}

.confirm-head p {
  margin: 0.35rem 0 0;
  color: var(--text-secondary);
  line-height: 1.5;
}

.confirm-body {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  padding: 1rem 1.1rem;
}

.details-list {
  display: grid;
  gap: 0.7rem;
}

.details-row {
  display: grid;
  grid-template-columns: 140px minmax(0, 1fr);
  gap: 0.75rem;
}

.details-row dt {
  color: var(--text-muted);
  font-size: 0.82rem;
}

.details-row dd {
  margin: 0;
  color: var(--text-primary);
  font-size: 0.86rem;
}

.preview-block {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.preview-label {
  font-size: 0.74rem;
  font-weight: 700;
  color: var(--text-muted);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.preview-block pre {
  margin: 0;
  padding: 0.85rem 0.9rem;
  overflow-x: auto;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-sidebar) 58%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.79rem;
  line-height: 1.55;
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
}

.confirm-warning,
.confirm-error {
  margin: 0;
  padding: 0.75rem 0.85rem;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  font-size: 0.84rem;
  line-height: 1.55;
}

.confirm-warning {
  background: color-mix(in srgb, var(--bg-warning) 55%, var(--bg-main));
  color: var(--text-primary);
}

.confirm-error {
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
  border-color: color-mix(in srgb, var(--bg-danger) 35%, var(--border-color));
  color: color-mix(in srgb, var(--bg-danger) 75%, var(--text-primary));
}

.confirm-actions {
  justify-content: flex-end;
  border-top: 1px solid var(--border-color);
}

@keyframes rotate-icon {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 880px) {
  .card-head,
  .summary-band,
  .confirm-head,
  .confirm-actions {
    flex-direction: column;
    align-items: stretch;
  }

  .health-group {
    justify-content: flex-start;
  }

  .action-button {
    justify-content: center;
  }
}

@media (max-width: 640px) {
  .settings-embeddings {
    padding: 1rem;
  }

  .details-row {
    grid-template-columns: 1fr;
    gap: 0.25rem;
  }
}
</style>
