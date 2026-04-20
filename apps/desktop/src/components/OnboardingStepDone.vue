<script setup lang="ts">
import { computed, inject, onUnmounted, ref, type Ref } from 'vue';
import { Check, ExternalLink, Sparkles } from 'lucide-vue-next';
import { toast } from 'vue-sonner';

import { DAEMON_BASE_URL } from '../lib/daemon-url';
import { useConfigStore } from '../stores/config';

const goBack = inject<() => void>('onboarding-go-back');
const markComplete = inject<() => void>('onboarding-mark-complete');
const modelSkipped = inject<Ref<boolean>>('onboarding-model-skipped');
const selectedModelName = inject<Ref<string | null>>('onboarding-selected-model');

const configStore = useConfigStore();

const finishing = ref(false);
let disposed = false;

const firstAgent = computed<string>(() => configStore.agentNames[0] ?? 'Unknown');

const memoryPath = '~/maenifold/memory';

async function probeDaemon(): Promise<boolean> {
  try {
    const res = await fetch(`${DAEMON_BASE_URL}/health`, {
      signal: AbortSignal.timeout(3000),
    });
    return res.ok;
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
    <div class="wizard-glow" aria-hidden="true" />

    <div
      class="relative flex h-12 w-12 items-center justify-center rounded-xl bg-success/10 text-success"
    >
      <Sparkles class="h-6 w-6 wizard-pulse" />
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
            <span class="text-sm font-medium text-text-primary">Memory</span>
            <span class="text-xs text-text-secondary font-mono">{{ memoryPath }}</span>
          </div>
        </li>
      </ul>
    </div>

    <div class="relative flex items-center justify-between gap-3 pt-2">
      <button
        type="button"
        :disabled="finishing"
        class="rounded-lg border border-border-subtle bg-bg-surface px-4 py-2 text-sm font-medium text-text-secondary transition-colors hover:bg-bg-elevated disabled:cursor-not-allowed disabled:opacity-50"
        @click="onBack"
      >
        ← Back
      </button>

      <div class="flex items-center gap-3">
        <button
          type="button"
          class="flex items-center gap-1.5 text-sm font-medium text-text-secondary transition-colors hover:text-text-primary"
          @click="onOpenDocs"
        >
          <span>Read documentation</span>
          <ExternalLink class="h-3.5 w-3.5" />
        </button>

        <button
          type="button"
          :disabled="finishing"
          class="rounded-lg border-none bg-brand px-6 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
          @click="onFinish"
        >
          {{ finishing ? 'Checking…' : 'Start using maenifold' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes wizardGlow {
  0%, 100% { opacity: 0.15; }
  50% { opacity: 0.3; }
}

@keyframes wizardPulse {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.05); }
}

.wizard-glow {
  position: absolute;
  inset: -2rem;
  z-index: 0;
  pointer-events: none;
  background: radial-gradient(
    circle at 30% 20%,
    var(--color-brand) 0%,
    transparent 60%
  );
  filter: blur(60px);
  opacity: 0.15;
  animation: wizardGlow 4s ease-in-out infinite;
}

.wizard-pulse {
  animation: wizardPulse 2s ease-in-out infinite;
  transform-origin: center;
}
</style>
