<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref, type Ref } from 'vue';
import { Brain, Loader2 } from 'lucide-vue-next';

import { Button } from '@/components/ui/button';
import { useMcpClient } from '../composables/useMcpClient';

interface ModelInfo {
  id: string;
  name: string;
  dimensions: number;
  maxTokens: number;
  sizeBytes: number;
  installed: boolean;
  isActive: boolean;
}

interface ModelListResponse {
  models: ModelInfo[];
  currentDimensions: number;
}

const advance = inject<() => void>('onboarding-advance');
const goBack = inject<() => void>('onboarding-go-back');
const modelSkipped = inject<Ref<boolean>>('onboarding-model-skipped');
const selectedModelName = inject<Ref<string | null>>('onboarding-selected-model');

const { callTool, connect, connected } = useMcpClient();

const loading = ref(true);
const loadError = ref<string | null>(null);
const downloadError = ref<string | null>(null);
const downloading = ref(false);
const models = ref<ModelInfo[]>([]);
const selectedId = ref<string | null>(null);

let disposed = false;

const selectedModel = computed<ModelInfo | null>(
  () => models.value.find((m) => m.id === selectedId.value) ?? null,
);

function formatSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${Math.round(bytes / 1_000_000)} MB`;
  return `${Math.round(bytes / 1_000)} KB`;
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
      const active = models.value.find((m) => m.isActive);
      const installed = models.value.find((m) => m.installed);
      selectedId.value = (active ?? installed ?? models.value[0]).id;
    }
  } catch (e) {
    if (disposed) return;
    loadError.value = e instanceof Error ? e.message : String(e);
  } finally {
    if (!disposed) {
      loading.value = false;
    }
  }
}

function selectModel(model: ModelInfo): void {
  if (downloading.value) return;
  selectedId.value = model.id;
}

async function onDownload(): Promise<void> {
  if (selectedModel.value === null) return;
  downloadError.value = null;

  // If the selected model is already installed or active, just advance — no download needed
  if (selectedModel.value.installed || selectedModel.value.isActive) {
    if (modelSkipped) modelSkipped.value = false;
    if (selectedModelName) selectedModelName.value = selectedModel.value.name;
    advance?.();
    return;
  }

  // Model not installed — download action doesn't exist yet, show clear error
  downloadError.value = `Download not yet available. Please install "${selectedModel.value.name}" manually from HuggingFace (${selectedModel.value.id}) and place the ONNX file in your maenifold models directory.`;
}

function onSkip(): void {
  if (downloading.value) return;
  if (modelSkipped) modelSkipped.value = true;
  if (selectedModelName) selectedModelName.value = null;
  advance?.();
}

function onBack(): void {
  if (downloading.value) return;
  goBack?.();
}

function onRetry(): void {
  void loadModels();
}

onMounted(() => {
  void loadModels();
});

onUnmounted(() => {
  disposed = true;
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-brand/10 text-brand">
      <Brain class="h-6 w-6" />
    </div>

    <div class="flex flex-col gap-2">
      <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
        Download embedding model
      </h1>
      <p class="text-sm text-text-secondary leading-relaxed">
        Embedding models convert text to vectors for semantic search in your knowledge graph.
      </p>
    </div>

    <div
      v-if="loading"
      class="flex items-center gap-3 rounded-xl border border-border-subtle bg-bg-surface p-4"
    >
      <Loader2 class="h-4 w-4 animate-spin text-brand" />
      <span class="text-sm text-text-secondary">Loading models…</span>
    </div>

    <div v-else-if="loadError" class="flex flex-col gap-2">
      <p class="text-danger text-sm">Failed to load models: {{ loadError }}</p>
      <Button variant="outline" size="sm" class="self-start" @click="onRetry">
        Retry
      </Button>
    </div>

    <div v-else class="flex flex-col gap-2">
      <span class="text-xs font-medium text-text-secondary">Available models</span>
      <div class="flex flex-col gap-2">
        <Button
          v-for="model in models"
          :key="model.id"
          type="button"
          :disabled="downloading"
          class="bg-bg-surface border border-border-subtle rounded-xl p-4 cursor-pointer text-left transition-colors hover:border-text-tertiary disabled:cursor-not-allowed disabled:opacity-60"
          :class="{
            'border-brand bg-brand/5': selectedId === model.id,
          }"
          @click="selectModel(model)"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="flex flex-col gap-1">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium text-text-primary">{{ model.name }}</span>
                <span
                  v-if="model.isActive"
                  class="text-2xs bg-success/10 text-success rounded px-1.5 py-0.5"
                >
                  active
                </span>
                <span
                  v-else-if="model.installed"
                  class="text-2xs bg-bg-elevated text-text-secondary rounded px-1.5 py-0.5"
                >
                  installed
                </span>
              </div>
              <span class="text-xs text-text-tertiary">
                {{ model.dimensions }}d · {{ model.maxTokens }} max tokens ·
                {{ formatSize(model.sizeBytes) }}
              </span>
            </div>
            <span
              class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full border"
              :class="
                selectedId === model.id
                  ? 'border-brand bg-brand'
                  : 'border-border-subtle bg-bg-surface'
              "
            >
              <span
                v-if="selectedId === model.id"
                class="h-1.5 w-1.5 rounded-full bg-primary-foreground"
              />
            </span>
          </div>
        </Button>
      </div>
    </div>

    <div
      v-if="!loading && !loadError"
      class="bg-warning/10 border border-warning rounded-xl p-4 text-sm text-text-secondary"
    >
      ⚠️ Changing dimensions later requires re-embedding all existing memories.
    </div>

    <p v-if="downloadError" class="text-danger text-sm">
      Download failed: {{ downloadError }}
    </p>

    <div class="flex items-center justify-between gap-3 pt-2">
      <Button variant="outline" :disabled="downloading" @click="onBack">
        ← Back
      </Button>

      <div class="flex items-center gap-2">
        <Button variant="outline" :disabled="downloading" @click="onSkip">
          Download later →
        </Button>

        <Button
          variant="default"
          class="bg-brand hover:bg-brand-hover text-primary-foreground px-6"
          :disabled="loading || !!loadError || selectedModel === null || downloading"
          @click="onDownload"
        >
          <Loader2 v-if="downloading" class="h-4 w-4 animate-spin" />
          <span>{{ selectedModel?.installed || selectedModel?.isActive ? 'Next →' : 'Download →' }}</span>
        </Button>
      </div>
    </div>
  </div>
</template>
