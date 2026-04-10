<script setup lang="ts">
import { computed } from 'vue';
import type { PermissionRequest } from '../lib/types';
import PermissionEntry from './PermissionEntry.vue';

const props = defineProps<{ request: PermissionRequest; totalPending?: number }>();
const emit = defineEmits<{ select: [optionId: string]; cancel: [] }>();

const queueHint = computed(() =>
  (props.totalPending ?? 0) > 1
    ? `Showing request 1 of ${props.totalPending}. The remaining requests will appear after you respond.`
    : null
);
</script>

<template>
  <div class="permission-overlay">
    <div class="permission-dialog">
      <div class="dialog-header">
        <span class="icon">🔐</span>
        <h3>Permission Required</h3>
      </div>
      <div class="dialog-content">
        <p v-if="queueHint" class="queue-hint">{{ queueHint }}</p>
        <PermissionEntry
          :request="request"
          @select="emit('select', $event)"
          @cancel="emit('cancel')"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.permission-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

@supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .permission-overlay {
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    background: rgba(0, 0, 0, 0.4);
  }
}

.permission-dialog {
  background: var(--bg-main);
  border-radius: 8px;
  box-shadow: 0 4px 20px color-mix(in srgb, var(--bg-main) 15%, transparent);
  max-width: 480px;
  width: 90%;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-color);
}

.dialog-header .icon {
  font-size: var(--text-xl);
}

.dialog-header h3 {
  margin: 0;
  font-size: var(--text-xl);
}

.dialog-content {
  padding: 1rem;
}

.queue-hint {
  margin: 0 0 1rem;
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
</style>
