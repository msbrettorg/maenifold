<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  compact: boolean;
  degradedMessage: string | null;
  ready: boolean;
  refreshDisabled: boolean;
  refreshing: boolean;
  statusLabel: string;
  statusMessage: string;
  statusTone: string;
  technicalDetail: string | null;
  warningMessage: string | null | undefined;
}>();

defineEmits<{
  refresh: [];
}>();
</script>

<template>
  <div class="preview-status" :class="[statusTone, { compact }]">
    <div class="preview-header">
      <div class="status-copy">
        <span class="status-indicator">
          <span class="status-bead" aria-hidden="true" />
          <span class="status-label">{{ statusLabel }}</span>
        </span>
        <span v-if="statusMessage" class="status-message">{{ statusMessage }}</span>
      </div>
      <Button variant="outline" size="sm" class="refresh-btn" :disabled="refreshDisabled" @click="$emit('refresh')">
        {{ refreshing ? 'Refreshing…' : 'Refresh' }}
      </Button>
    </div>

    <p v-if="warningMessage" class="warning-note" role="status" aria-live="polite">{{ warningMessage }}</p>
    <p v-if="degradedMessage" class="degraded-note">{{ degradedMessage }}</p>
    <p v-if="technicalDetail" class="technical-detail">{{ technicalDetail }}</p>
  </div>
</template>

<style scoped>
.preview-status {
  display: grid;
  gap: 0.65rem;
}

.preview-header {
  display: flex;
  flex-wrap: wrap;
  gap: 0.65rem;
  align-items: center;
  justify-content: space-between;
}

.status-copy {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  align-items: center;
}

.status-indicator {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

.status-bead {
  width: 6px;
  height: 6px;
  border-radius: 4px;
  background: var(--text-dim);
}

.status-label {
  font-size: var(--text-tiny);
  color: var(--color-text-primary);
}

.status-message,
.warning-note,
.degraded-note,
.technical-detail {
  font-size: var(--text-caption);
}

.status-message,
.technical-detail {
  color: var(--color-text-secondary);
}

.warning-note,
.degraded-note {
  color: var(--color-text-primary);
}

.warning-note,
.degraded-note,
.technical-detail {
  margin: 0;
}

.warning-note {
  padding: 0.55rem 0.65rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--bg-warning) 20%, var(--surface-container));
}

.technical-detail {
  word-break: break-word;
}

.refresh-btn {
  border: none;
  border-radius: 4px;
  background: var(--surface-container);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: var(--text-tiny);
  font-weight: 400;
  padding: 0.35rem 0.65rem;
  white-space: nowrap;
}

.refresh-btn:disabled {
  cursor: wait;
  opacity: 0.7;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--surface-high);
}

.ready .status-bead {
  background: var(--sage);
}

.pending .status-bead,
.idle .status-bead {
  background: var(--text-dim);
}

.offline .status-bead {
  background: var(--amber);
}

.error .status-bead {
  background: var(--terracotta);
}
</style>
