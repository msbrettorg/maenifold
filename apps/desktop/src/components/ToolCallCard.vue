<script setup lang="ts">
// Lifted from formulahendry/acp-ui (MIT) — see LICENSE-acp-ui
// https://github.com/formulahendry/acp-ui
// Adaptation notes: Maenifold adaptation: replace scoped CSS classes with Tailwind design tokens.
// Maenifold adaptation: expose an inert details slot so wrappers can render Maenifold-only extensions without changing lifted control flow.

import { computed } from 'vue';
import type { ToolCallInfo } from '../lib/types';
import { autoPermissionStatuses } from '../lib/permission-ledger/audit-events';

const props = defineProps<{
  toolCall: ToolCallInfo;
}>();

const statusClass = computed(() => {
  switch (props.toolCall.status) {
    case 'pending': return 'border-l-warning bg-warning/10';
    case 'in_progress': return 'border-l-brand bg-brand/10';
    case 'completed': return 'border-l-success bg-success/10';
    case 'failed': return 'border-l-danger bg-danger/10';
    default: return 'border-l-border-subtle bg-surface-high';
  }
});

const statusClassName = computed(() => {
  switch (props.toolCall.status) {
    case 'pending': return 'status-pending';
    case 'in_progress': return 'status-running';
    case 'completed': return 'status-completed';
    case 'failed': return 'status-failed';
    default: return '';
  }
});

const statusIcon = computed(() => {
  switch (props.toolCall.status) {
    case 'pending': return '⏳';
    case 'in_progress': return '⚙️';
    case 'completed': return '✅';
    case 'failed': return '❌';
    default: return '🔧';
  }
});

const kindIcon = computed(() => {
  switch (props.toolCall.kind) {
    case 'read': return '📖';
    case 'edit': return '✏️';
    case 'write': return '📝';
    case 'execute': return '▶️';
    default: return '🔧';
  }
});

const autoPermissionStatus = computed(() => autoPermissionStatuses.value[props.toolCall.toolCallId] ?? null);
</script>

<template>
  <div
    :class="['tool-call-card my-2 rounded-md border border-border-subtle border-l-4 p-3 text-sm', statusClass, statusClassName]"
  >
    <div class="tool-header flex items-center gap-2">
      <span class="kind-icon text-base">{{ kindIcon }}</span>
      <span class="tool-title flex-1 font-medium text-text-primary">{{ toolCall.title }}</span>
      <span
        v-if="autoPermissionStatus"
        class="rounded border border-border-subtle px-2 py-0.5 text-xs font-semibold text-text-primary"
      >
        {{ autoPermissionStatus === 'auto-allowed' ? 'Auto-allowed' : 'Auto-denied' }}
      </span>
      <span v-else class="status-icon text-sm">{{ statusIcon }}</span>
    </div>

    <div v-if="toolCall.locations?.length" class="tool-locations mt-2 flex flex-col gap-1 pl-6">
      <div
        v-for="(loc, index) in toolCall.locations"
        :key="index"
        class="location break-all font-mono text-xs text-text-secondary"
      >
        📁 {{ loc.path }}
      </div>
    </div>

    <slot name="details" :tool-call="toolCall" />
  </div>
</template>
