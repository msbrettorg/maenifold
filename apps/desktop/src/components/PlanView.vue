<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Ruler } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import type { PlanEntry } from '../lib/types';

const props = defineProps<{
  entries: PlanEntry[];
}>();

const isExpanded = ref(true);

const entryCount = computed(() => props.entries.length);

const completedCount = computed(
  () => props.entries.filter((entry) => entry.status === 'completed').length
);

const statusLabelMap: Record<PlanEntry['status'], string> = {
  pending: 'Pending',
  in_progress: 'In Progress',
  completed: 'Completed',
};

const statusIconMap: Record<PlanEntry['status'], string> = {
  pending: '○',
  in_progress: '◐',
  completed: '✓',
};

const statusColorMap: Record<PlanEntry['status'], string> = {
  pending: 'text-[var(--color-status-pending)]',
  in_progress: 'text-[var(--color-status-active)] animate-plan-pulse',
  completed: 'text-[var(--color-status-completed)]',
};

const priorityStyle: Record<string, Record<string, string>> = {
  high: {
    background: 'color-mix(in srgb, var(--color-status-abandoned) 12%, var(--color-bg-app))',
    color: 'var(--color-danger)',
  },
  medium: {
    background: 'color-mix(in srgb, var(--color-status-stuck) 16%, var(--color-bg-app))',
    color: 'color-mix(in srgb, var(--color-status-stuck) 72%, var(--color-text-primary))',
  },
  low: {
    background: 'color-mix(in srgb, var(--color-status-pending) 14%, var(--color-bg-app))',
    color: 'var(--color-text-secondary)',
  },
};

watch(
  () => props.entries,
  () => {
    isExpanded.value = true;
  },
  { deep: true }
);

function toggleExpanded(): void {
  isExpanded.value = !isExpanded.value;
}
</script>

<template>
  <section class="mb-4 rounded-md border border-border-subtle bg-bg-surface overflow-hidden">
    <Button
      type="button"
      variant="ghost"
      class="flex items-center justify-between gap-3 w-full py-3.5 px-4 bg-transparent text-text-text-primary cursor-pointer text-left hover:bg-bg-surface h-auto"
      :aria-expanded="isExpanded"
      @click="toggleExpanded"
    >
      <div class="flex items-center gap-2.5 min-w-0">
        <span class="shrink-0"><Ruler class="size-4" :stroke-width="1.5" /></span>
        <span class="font-[var(--weight-title)]">Agent Plan</span>
        <span class="text-text-secondary text-[length:var(--text-caption)]">
          {{ completedCount }}/{{ entryCount }} completed
        </span>
      </div>
      <span class="text-text-secondary text-[length:var(--text-caption)]">{{ isExpanded ? '▲' : '▼' }}</span>
    </Button>

    <div v-if="isExpanded" class="border-t border-border-subtle pt-3 px-4 pb-4">
      <ol class="flex flex-col gap-3 m-0 p-0 list-none">
        <li
          v-for="(entry, index) in entries"
          :key="`${index}-${entry.content}`"
          class="flex items-start gap-3"
        >
          <span
            :class="[
              'inline-flex items-center justify-center w-6 h-6 mt-0.5 rounded-sm bg-bg-surface text-[length:var(--text-body-sm)] font-[var(--weight-title)] shrink-0',
              statusColorMap[entry.status],
            ]"
            :title="statusLabelMap[entry.status]"
            :aria-label="statusLabelMap[entry.status]"
          >
            {{ statusIconMap[entry.status] }}
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 mb-1">
              <!-- audit-ok: dynamic-binding inline color-mix() expressions for priority badge backgrounds; cannot tokenize per-priority blends -->
              <span
                class="inline-flex items-center py-0.5 px-2 rounded-sm text-tiny font-[var(--weight-title)]"
                :style="priorityStyle[entry.priority]"
              >
                {{ entry.priority }}
              </span>
              <span class="text-text-secondary text-[length:var(--text-caption)]">
                {{ statusLabelMap[entry.status] }}
              </span>
            </div>
            <p class="m-0 text-text-text-primary leading-snug whitespace-pre-wrap">{{ entry.content }}</p>
          </div>
        </li>
      </ol>
    </div>
  </section>
</template>

<!-- audit-ok: keyframe-animation @keyframes pulse used by .animate-plan-pulse for in-progress status indicator. -->
<style scoped>
.animate-plan-pulse {
  animation: pulse 1.2s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.45;
  }
}
</style>
