<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  isLoading: boolean;
  error: string;
  nodeCount: number;
}>();

defineEmits<{ retry: [] }>();
</script>

<template>
  <div
    class="relative flex-1 overflow-hidden bg-bg-app"
    role="region"
    aria-label="Knowledge graph canvas"
  >
    <div
      v-if="error && !isLoading"
      class="absolute inset-0 z-[2] flex flex-col items-center justify-center bg-brand/5"
      role="alert"
    >
      <div class="mb-4 text-display text-text-secondary opacity-30">◎</div>
      <p class="mb-1 font-display text-lg text-text-secondary">No graph connection</p>
      <p class="mb-5 text-caption text-text-secondary">Launch maenifold to explore the knowledge graph.</p>
      <Button variant="outline" size="sm" @click="$emit('retry')">Retry</Button>
    </div>

    <div
      v-else-if="isLoading"
      class="absolute inset-0 z-[2] flex flex-col items-center justify-center"
      role="status"
      aria-live="polite"
    >
      <div class="mb-3 h-8 w-8 animate-spin rounded-lg border-2 border-border-subtle border-t-brand" />
      <p class="text-caption text-text-secondary">Building knowledge map…</p>
      <p v-if="nodeCount > 0" class="mt-1 text-[0.6875rem] text-text-secondary tabular-nums">{{ nodeCount }} nodes discovered</p>
    </div>

    <div class="relative w-full h-full bg-transparent">
      <slot />
    </div>

    <div
      v-if="!isLoading && !error"
      class="pointer-events-none absolute bottom-0 left-0 right-0 z-[1] px-4 py-2 text-center text-[0.6875rem] text-text-secondary"
    >
      Drag to pan · Scroll to zoom · Click a node for details
    </div>
  </div>
</template>
