<script setup lang="ts">
import { inject } from 'vue';
import { Sparkles } from 'lucide-vue-next';

const advance = inject<() => void>('onboarding-advance');

function onGetStarted(): void {
  advance?.();
}

const previewItems = [
  '🔗 Daemon connection',
  '🤖 Configure your first agent',
  '🧠 Download embedding model',
];
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-brand/10 text-brand">
      <Sparkles class="h-6 w-6" />
    </div>

    <div class="flex flex-col gap-2">
      <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
        Welcome to maenifold
      </h1>
      <p class="text-sm text-text-secondary leading-relaxed">
        Your personal knowledge engine and agent orchestrator. Let's get you set up in a
        few steps.
      </p>
    </div>

    <ul class="flex flex-col gap-2">
      <li
        v-for="item in previewItems"
        :key="item"
        class="rounded-lg border border-border-subtle bg-bg-surface p-3 text-sm text-text-secondary"
      >
        {{ item }}
      </li>
    </ul>

    <div class="pt-2">
      <button
        type="button"
        class="rounded-lg border-none bg-brand px-6 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-brand-hover"
        @click="onGetStarted"
      >
        Get started →
      </button>
    </div>
  </div>
</template>
