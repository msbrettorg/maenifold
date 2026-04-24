<script setup lang="ts">
import { computed, type Component } from 'vue';

import { Badge } from '@/components/ui/badge';

const props = defineProps<{
  displayLabel: string;
  failureSummary?: string | null;
  statusAppearance: string;
  typeIcon: Component;
  typeLabel: string;
}>();

const DOT_CLASS: Record<string, string> = {
  pending: 'bg-status-pending',
  cancelled: 'bg-status-pending',
  active: 'bg-status-active',
  stuck: 'bg-status-stuck',
  completed: 'bg-status-completed',
  abandoned: 'bg-status-abandoned',
};

const dotClass = computed(() => DOT_CLASS[props.statusAppearance] ?? 'bg-muted-foreground');
const isActive = computed(() => props.statusAppearance === 'active');
</script>

<template>
  <div class="flex items-center justify-between gap-2 px-3 pt-2.5 pb-1">
    <div class="flex min-w-0 items-center gap-1.5">
      <span class="relative inline-flex h-1.5 w-1.5 shrink-0 items-center justify-center">
        <span
          v-if="isActive"
          class="absolute inline-flex h-full w-full animate-ping rounded-full bg-status-active opacity-75"
          aria-hidden="true"
        />
        <span
          class="relative inline-block h-1.5 w-1.5 rounded-full"
          :class="[dotClass, isActive ? 'animate-pulse' : '']"
          aria-hidden="true"
        />
      </span>
      <span class="truncate text-sm font-semibold text-foreground">{{ displayLabel }}</span>
      <span
        v-if="failureSummary"
        class="shrink-0 cursor-help text-xs leading-none text-status-abandoned"
        :title="failureSummary"
        aria-label="Node failure reason available"
      >
        ⚠
      </span>
    </div>
    <Badge variant="secondary" class="shrink-0 gap-1 px-1.5 py-0.5 text-xs font-normal">
      <component :is="typeIcon" class="size-3.5" :stroke-width="1.5" />
      <span class="truncate">{{ typeLabel }}</span>
    </Badge>
  </div>
</template>
