<script setup lang="ts">
import { computed } from 'vue';

import { Badge } from '@/components/ui/badge';
import type { PipelineStatus } from '@/lib/pipeline-types-edge';

const props = defineProps<{ status: PipelineStatus | null | undefined }>();

const DOT_CLASS: Record<PipelineStatus, string> = {
  active: 'bg-status-active',
  running: 'bg-status-active',
  completed: 'bg-status-completed',
  stuck: 'bg-status-stuck',
  abandoned: 'bg-status-abandoned',
  paused: 'bg-status-pending',
  cancelled: 'bg-status-pending',
};

const dotClass = computed(() =>
  props.status ? (DOT_CLASS[props.status] ?? 'bg-muted-foreground') : 'bg-muted-foreground',
);
</script>

<template>
  <Badge
    v-if="status"
    variant="secondary"
    class="absolute top-3 left-3 z-[7] gap-2 px-2.5 py-1.5"
    :aria-label="`Pipeline status: ${status}`"
  >
    <span class="h-1.5 w-1.5 shrink-0 rounded-full" :class="dotClass" aria-hidden="true" />
    <span>{{ status }}</span>
  </Badge>
</template>
