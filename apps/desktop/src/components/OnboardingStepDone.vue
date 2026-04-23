<script setup lang="ts">
import { computed, inject, onUnmounted, ref, type Ref } from 'vue';
import { Check, ExternalLink, Sparkles } from 'lucide-vue-next';
import { toast } from 'vue-sonner';

import { Button } from '@/components/ui/button';
import { useConfigStore } from '../stores/config';

const goBack = inject<() => void>('onboarding-go-back');
const markComplete = inject<() => void>('onboarding-mark-complete');
const modelSkipped = inject<Ref<boolean>>('onboarding-model-skipped');
const selectedModelName = inject<Ref<string | null>>('onboarding-selected-model');

const configStore = useConfigStore();

const finishing = ref(false);
let disposed = false;

const firstAgent = computed<string>(() => configStore.agentNames[0] ?? 'Unknown');

const memoryPath = computed<string>(
  () => configStore.configPath || '~/.config/maenifold/maenifold.json',
);

async function probeDaemon(): Promise<boolean> {
  try {
    await configStore.loadConfig();
    return configStore.error === null;
  } catch {
    return false;
  }
}

async function onFinish(): Promise<void> {
  if (finishing.value) return;
  finishing.value = true;
  try {
    const alive = await probeDaemon();
    if (disposed) return;
    if (!alive) {
      toast.warning('Daemon is not responding. Please start it before continuing.');
      return;
    }
    markComplete?.();
  } finally {
    if (!disposed) {
      finishing.value = false;
    }
  }
}

function onBack(): void {
  goBack?.();
}

function onOpenDocs(): void {
  window.open('https://github.com/msbrettorg/maenifold', '_blank');
}

onUnmounted(() => {
  disposed = true;
});
</script>

<template>
  <div class="relative flex flex-col gap-6">
    <!-- Animated glow backdrop -->
    <div
      aria-hidden="true"
      class="absolute -inset-8 z-0 pointer-events-none bg-brand/20 blur-3xl rounded-full animate-pulse"
    />

    <div
      class="relative flex h-12 w-12 items-center justify-center rounded-xl bg-success/10 text-success"
    >
      <Sparkles class="h-6 w-6 animate-pulse" />
    </div>

    <div class="relative flex flex-col gap-2">
      <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
        ✨ You're all set! ✨
      </h1>
      <p class="text-sm text-text-secondary leading-relaxed">
        Your maenifold environment is ready. Here's a summary of what we configured.
      </p>
    </div>

    <!-- Summary card -->
    <div class="relative bg-bg-elevated border border-border-card rounded-xl p-5">
      <ul class="flex flex-col gap-3">
        <li class="flex items-start gap-3">
          <span
            class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-success text-primary-foreground"
          >
            <Check class="h-3 w-3" />
          </span>
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium text-text-primary">Daemon</span>
            <span class="text-xs text-text-secondary">Connected</span>
          </div>
        </li>

        <li class="flex items-start gap-3">
          <span
            class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-success text-primary-foreground"
          >
            <Check class="h-3 w-3" />
          </span>
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium text-text-primary">Agent</span>
            <span class="text-xs text-text-secondary">{{ firstAgent }}</span>
          </div>
        </li>

        <li class="flex items-start gap-3">
          <template v-if="modelSkipped">
            <span
              class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-warning/20 text-warning text-xs font-bold"
              aria-hidden="true"
            >
              ⚠
            </span>
            <div class="flex flex-col gap-0.5">
              <span class="text-sm font-medium text-text-primary">Model</span>
              <span class="text-xs text-warning">Download later (skipped)</span>
            </div>
          </template>
          <template v-else>
            <span
              class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-success text-primary-foreground"
            >
              <Check class="h-3 w-3" />
            </span>
            <div class="flex flex-col gap-0.5">
              <span class="text-sm font-medium text-text-primary">Model</span>
              <span class="text-xs text-text-secondary">
                {{ selectedModelName ?? 'Configured' }}
              </span>
            </div>
          </template>
        </li>

        <li class="flex items-start gap-3">
          <span
            class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-success text-primary-foreground"
          >
            <Check class="h-3 w-3" />
          </span>
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium text-text-primary">Config</span>
            <span class="text-xs text-text-secondary font-mono">{{ memoryPath }}</span>
          </div>
        </li>
      </ul>
    </div>

    <div class="relative flex items-center justify-between gap-3 pt-2">
      <Button
        type="button"
        variant="outline"
        :disabled="finishing"
        @click="onBack"
      >
        ← Back
      </Button>

      <div class="flex items-center gap-3">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          @click="onOpenDocs"
        >
          <span>Read documentation</span>
          <ExternalLink class="h-3.5 w-3.5" />
        </Button>

        <Button
          type="button"
          variant="default"
          size="lg"
          :disabled="finishing"
          @click="onFinish"
        >
          {{ finishing ? 'Checking…' : 'Start using maenifold' }}
        </Button>
      </div>
    </div>
  </div>
</template>

