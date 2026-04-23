<script setup lang="ts">
import { computed } from 'vue';

import { STATUS_COLORS } from '@/lib/pipeline-status-tokens';
import type { PipelineStatus } from '@/lib/pipeline-types';

const props = defineProps<{ status: PipelineStatus | null | undefined }>();

const badge = computed(() => {
  if (!props.status) return null;
  return {
    label: props.status,
    color: STATUS_COLORS[props.status] ?? 'var(--text-secondary)',
  };
});
</script>

<template>
  <div v-if="badge" class="composer-status-badge" :aria-label="`Pipeline status: ${badge.label}`">
    <!-- audit-ok: dynamic-binding (status dot color computed from badge state) -->
    <span class="composer-status-dot" :style="{ backgroundColor: badge.color }" aria-hidden="true" />
    <span>{{ badge.label }}</span>
  </div>
</template>

<style scoped>
.composer-status-badge {
  position: absolute;
  top: 0.75rem;
  left: 0.75rem;
  z-index: 7;
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem 0.6rem;
  border-radius: var(--radius-4);
  background: var(--surface-container);
  color: var(--text);
  font-size: var(--text-xs);
}

.composer-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
</style>
