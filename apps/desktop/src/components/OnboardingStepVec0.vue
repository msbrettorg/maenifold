<script setup lang="ts">
import { inject, onMounted } from 'vue';
import { DatabaseZap, Loader2 } from 'lucide-vue-next';

import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { formatModelSize } from '@/composables/useEmbeddingsActions';
import type { useOnboardingVec0Download } from '@/composables/useOnboardingVec0Download';

type Vec0DownloadState = ReturnType<typeof useOnboardingVec0Download>;

const advance = inject<() => void>('onboarding-advance');
const goBack = inject<() => void>('onboarding-go-back');
const vec0 = inject<Vec0DownloadState>('onboarding-vec0');

function onBack(): void {
  if (vec0?.downloading.value) return;
  goBack?.();
}

async function onDownload(): Promise<void> {
  if (!vec0) return;
  await vec0.downloadVec0();
  if (vec0.isInstalled.value) {
    advance?.();
  }
}

function onSkip(): void {
  if (vec0?.downloading.value) return;
  advance?.();
}

function onRetry(): void {
  void vec0?.loadStatus();
}

onMounted(() => {
  void vec0?.loadStatus();
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-brand/10 text-brand">
      <DatabaseZap class="h-6 w-6" />
    </div>

    <div class="flex flex-col gap-2">
      <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
        Download vec0 extension
      </h1>
      <p class="text-sm leading-relaxed text-text-secondary">
        Vec0 powers local vector indexes inside SQLite. Download the native extension for this
        machine before you start syncing semantic embeddings.
      </p>
    </div>

    <div
      v-if="vec0?.loading.value"
      class="flex items-center gap-3 rounded-xl border border-border-subtle bg-bg-surface p-4"
    >
      <Loader2 class="h-4 w-4 animate-spin text-brand" />
      <span class="text-sm text-text-secondary">Checking vec0 runtime…</span>
    </div>

    <div v-else-if="vec0?.loadError.value" class="flex flex-col gap-2">
      <p class="text-sm text-danger">Failed to inspect vec0: {{ vec0.loadError.value }}</p>
      <Button variant="outline" size="sm" class="self-start" @click="onRetry">
        Retry
      </Button>
    </div>

    <div v-else-if="vec0?.status.value" class="flex flex-col gap-4">
      <div class="rounded-xl border border-border-subtle bg-bg-surface p-4">
        <dl class="grid gap-3 text-sm sm:grid-cols-2">
          <div class="flex flex-col gap-1">
            <dt class="text-xs font-medium uppercase tracking-wide text-text-tertiary">Platform</dt>
            <dd class="text-text-primary">
              {{ vec0.status.value.platform }}/{{ vec0.status.value.arch }}
            </dd>
          </div>
          <div class="flex flex-col gap-1">
            <dt class="text-xs font-medium uppercase tracking-wide text-text-tertiary">Target</dt>
            <dd class="font-mono text-xs text-text-secondary">
              {{ vec0.targetPath.value ?? 'Unavailable' }}
            </dd>
          </div>
          <div class="flex flex-col gap-1">
            <dt class="text-xs font-medium uppercase tracking-wide text-text-tertiary">Release asset</dt>
            <dd class="font-mono text-xs text-text-secondary">
              {{ vec0.status.value.assetPattern ?? 'Unsupported platform' }}
            </dd>
          </div>
          <div class="flex flex-col gap-1">
            <dt class="text-xs font-medium uppercase tracking-wide text-text-tertiary">Status</dt>
            <dd class="text-text-primary">
              {{ vec0.isInstalled.value ? 'Installed' : 'Missing' }}
            </dd>
          </div>
        </dl>
      </div>

      <div
        v-if="vec0.downloading.value"
        class="flex flex-col gap-2 rounded-xl border border-border-subtle bg-bg-surface p-4"
      >
        <div class="flex items-center justify-between gap-3">
          <span class="text-sm font-medium text-text-primary">Downloading vec0</span>
          <span v-if="vec0.hasDownloadProgress.value" class="text-xs text-text-secondary">
            {{ Math.round(vec0.downloadPercent.value) }}%
          </span>
        </div>

        <Progress
          v-if="vec0.hasDownloadProgress.value"
          :model-value="vec0.downloadPercent.value"
          class="h-2"
        />
        <div v-else class="h-2 overflow-hidden rounded-full bg-muted">
          <div class="h-full w-1/3 animate-pulse rounded-full bg-primary" />
        </div>

        <p class="text-xs text-text-secondary">
          {{
            vec0.hasDownloadProgress.value
              ? `Downloaded ${formatModelSize(vec0.downloadProgress.value?.current ?? 0)} of ${formatModelSize(vec0.downloadProgress.value?.total ?? 0)}`
              : 'Preparing download…'
          }}
        </p>
      </div>

      <p v-if="vec0.downloadError.value" class="text-sm text-danger">
        Download failed: {{ vec0.downloadError.value }}
      </p>
    </div>

    <div class="flex items-center justify-between gap-3 pt-2">
      <Button variant="outline" :disabled="vec0?.downloading.value" @click="onBack">
        ← Back
      </Button>

      <div class="flex items-center gap-2">
        <Button variant="outline" :disabled="vec0?.downloading.value" @click="onSkip">
          Continue →
        </Button>
        <Button
          variant="default"
          class="bg-brand px-6 text-primary-foreground hover:bg-brand-hover"
          :disabled="
            vec0?.loading.value
              || !!vec0?.loadError.value
              || !vec0?.status.value?.supported
              || vec0?.downloading.value
          "
          @click="onDownload"
        >
          <Loader2 v-if="vec0?.downloading.value" class="h-4 w-4 animate-spin" />
          <span>{{ vec0?.isInstalled.value ? 'Next →' : 'Download →' }}</span>
        </Button>
      </div>
    </div>
  </div>
</template>
