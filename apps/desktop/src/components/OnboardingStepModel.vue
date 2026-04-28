<script setup lang="ts">
import { inject, onMounted, type Ref } from 'vue';
import { Brain, Loader2 } from 'lucide-vue-next';

import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  useOnboardingModelDownload,
  type OnboardingModelInfo,
} from '../composables/useOnboardingModelDownload';
import { formatModelSize } from '../composables/useEmbeddingsActions';

const advance = inject<() => void>('onboarding-advance');
const goBack = inject<() => void>('onboarding-go-back');
const modelSkipped = inject<Ref<boolean>>('onboarding-model-skipped');
const selectedModelName = inject<Ref<string | null>>('onboarding-selected-model');

const {
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
} = useOnboardingModelDownload({
  advance,
  modelSkipped,
  selectedModelName,
});

function formatSize(bytes: number): string {
  return formatModelSize(bytes);
}

function onSelectModel(model: OnboardingModelInfo): void {
  selectModel(model);
}

async function onDownload(): Promise<void> {
  await downloadSelectedModel();
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
      <p
        v-if="models.length === 0"
        class="text-sm text-text-secondary"
      >
        No embedding models available. Click "Download later →" to skip.
      </p>
      <div class="flex flex-col gap-2">
        <Button
          v-for="model in models"
          :key="model.id"
          type="button"
          :disabled="downloading"
           class="bg-bg-surface border border-border-subtle rounded-xl p-4 cursor-pointer text-left transition-colors hover:border-text-tertiary disabled:cursor-not-allowed disabled:opacity-60"
           :class="{
             'border-brand bg-brand/5': selectedModel?.id === model.id,
           }"
           @click="onSelectModel(model)"
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
                 selectedModel?.id === model.id
                   ? 'border-brand bg-brand'
                   : 'border-border-subtle bg-bg-surface'
               "
             >
               <span
                 v-if="selectedModel?.id === model.id"
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

    <div
      v-if="downloading && selectedModel"
      class="flex flex-col gap-2 rounded-xl border border-border-subtle bg-bg-surface p-4"
    >
      <div class="flex items-center justify-between gap-3">
        <span class="text-sm font-medium text-text-primary">
          Downloading {{ selectedModel.name }}
        </span>
        <span v-if="hasDownloadProgress" class="text-xs text-text-secondary">
          {{ Math.round(downloadPercent) }}%
        </span>
      </div>

      <Progress
        v-if="hasDownloadProgress"
        :model-value="downloadPercent"
        class="h-2"
      />
      <div
        v-else
        class="h-2 overflow-hidden rounded-full bg-muted"
      >
        <div class="h-full w-1/3 rounded-full bg-primary animate-pulse" />
      </div>

      <p class="text-xs text-text-secondary">
        {{
          hasDownloadProgress
            ? `Downloaded ${formatSize(selectedModelProgress?.current ?? 0)} of ${formatSize(selectedModelProgress?.total ?? 0)}`
            : 'Preparing download…'
        }}
      </p>
    </div>

    <p v-if="downloadError" class="text-danger text-sm">
      Download failed: {{ downloadError }}
    </p>

    <div class="flex items-center justify-between gap-3 pt-2">
      <Button variant="outline" :disabled="downloading" @click="onBack">
        ← Back
      </Button>

      <div class="flex flex-col items-end gap-2">
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
            <span>
              {{
                downloading
                  ? 'Downloading…'
                  : selectedModel?.installed || selectedModel?.isActive
                    ? 'Next →'
                    : 'Download →'
              }}
            </span>
          </Button>
        </div>

        <p
          v-if="selectedModel === null && models.length > 0"
          class="mt-2 text-xs text-text-tertiary"
        >
          Select a model above first.
        </p>
      </div>
    </div>
  </div>
</template>
