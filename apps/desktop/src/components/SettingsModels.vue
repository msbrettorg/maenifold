<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useMcpClient } from '../composables/useMcpClient';

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
}

interface ModelListResponse {
  models: ModelInfo[];
  currentDimensions: number;
}

const { callTool, connect, connected } = useMcpClient();

const loading = ref(true);
const error = ref('');
const status = ref<ModelStatus | null>(null);
const models = ref<ModelInfo[]>([]);
const currentDimensions = ref<number>(0);

const activeModel = computed(() => models.value.find((m) => m.isActive) ?? null);

const hasDimensionWarning = computed(() => {
  if (!currentDimensions.value) return false;
  return models.value.some((m) => m.installed && m.dimensions !== currentDimensions.value);
});

function formatSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${Math.round(bytes / 1_000_000)} MB`;
  return `${Math.round(bytes / 1_000)} KB`;
}

async function loadData() {
  loading.value = true;
  error.value = '';

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
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void loadData();
});
</script>

<template>
  <section class="settings-section" role="region" aria-label="Models">
    <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
      <div class="section-header">
        <div>
          <h3>Models</h3>
          <p>Embedding models for semantic search and concept similarity.</p>
        </div>
      </div>

      <p v-if="error" class="message error">{{ error }}</p>

      <div v-if="loading" class="card">Loading model information…</div>

      <template v-else-if="!error">
        <div class="card">
          <div class="card-header">
            <div>
              <strong>Current model</strong>
              <p v-if="activeModel">
                {{ activeModel.name }} — {{ activeModel.dimensions }}d,
                {{ activeModel.maxTokens }} max tokens
              </p>
              <p v-else-if="status?.loaded">
                {{ status.model?.filename }} — {{ status.model?.dimensions }}d,
                {{ status.model?.maxTokens }} max tokens
              </p>
              <p v-else>No model loaded</p>
            </div>
          </div>
        </div>

        <div v-for="model in models" :key="model.id" class="card">
          <div class="card-header">
            <div>
              <strong>{{ model.name }}</strong>
              <p>{{ model.dimensions }}d · {{ formatSize(model.sizeBytes) }} · {{ model.license }}</p>
            </div>
            <span
              class="status-badge"
              :class="{ active: model.isActive, installed: model.installed && !model.isActive }"
            >
              {{ model.isActive ? 'Active' : model.installed ? 'Installed' : 'Not installed' }}
            </span>
          </div>
        </div>

        <div v-if="hasDimensionWarning" class="warning-banner">
          Switching to a model with different dimensions requires regenerating all embeddings.
        </div>
      </template>
    </div>
  </section>
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

.card-header {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-start;
}

.section-header h3,
.section-header p,
.message,
.card-header p {
  margin: 0;
}

.section-header h3 {
  font-family: var(--font-display);
  font-weight: 500;
}

.section-header p,
.card-header p {
  color: var(--color-text-secondary);
}

.card {
  border-radius: var(--radius);
  padding: 1rem;
  background: var(--color-bg-elevated);
  margin-top: 0.5rem;
}

.message.error {
  color: var(--color-danger);
}

.status-badge {
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius);
  font-size: 0.7rem;
  font-weight: 600;
  white-space: nowrap;
}

.status-badge.active {
  background: rgba(14, 156, 87, 0.15);
  color: var(--color-success);
}

.status-badge.installed {
  background: var(--color-bg-elevated);
  color: var(--color-text-secondary);
}

.warning-banner {
  margin-top: 0.75rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius);
  background: rgba(232, 145, 10, 0.12);
  color: var(--color-warning);
  font-size: 0.8rem;
}

@media (max-width: 640px) {
  .section-header,
  .card-header {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
