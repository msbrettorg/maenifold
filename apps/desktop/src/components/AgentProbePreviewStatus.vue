<script setup lang="ts">
defineProps<{
  compact: boolean;
  currentDefaults: string[];
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
        <span class="status-pill">{{ statusLabel }}</span>
        <span class="status-message">{{ statusMessage }}</span>
      </div>
      <button type="button" class="refresh-btn" :disabled="refreshDisabled" @click="$emit('refresh')">
        {{ refreshing ? 'Refreshing…' : 'Refresh' }}
      </button>
    </div>

    <p v-if="warningMessage" class="warning-note" role="status" aria-live="polite">{{ warningMessage }}</p>
    <p v-if="degradedMessage" class="degraded-note">{{ degradedMessage }}</p>
    <p v-if="technicalDetail" class="technical-detail">{{ technicalDetail }}</p>

    <div v-if="ready && currentDefaults.length" class="default-summary">
      {{ currentDefaults.join(' · ') }}
    </div>
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

.status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 5.5rem;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  font-size: var(--text-tiny);
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.status-message,
.default-summary,
.warning-note,
.degraded-note,
.technical-detail {
  font-size: var(--text-caption);
}

.status-message,
.technical-detail {
  color: var(--muted-foreground);
}

.default-summary,
.warning-note,
.degraded-note {
  color: var(--foreground);
}

.warning-note,
.degraded-note,
.technical-detail {
  margin: 0;
}

.warning-note {
  padding: 0.55rem 0.65rem;
  border: 1px solid color-mix(in srgb, var(--bg-warning) 55%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-warning) 30%, transparent);
}

.technical-detail {
  word-break: break-word;
}

.refresh-btn {
  border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--card) 82%, transparent);
  color: var(--muted-foreground);
  cursor: pointer;
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  padding: 0.25rem 0.65rem;
  white-space: nowrap;
}

.refresh-btn:disabled {
  cursor: wait;
  opacity: 0.7;
}

.compact .default-summary {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ready .status-pill {
  background: color-mix(in srgb, var(--ring) 45%, transparent);
  color: var(--ring);
}

.pending .status-pill,
.idle .status-pill {
  background: color-mix(in srgb, var(--card) 82%, transparent);
  color: var(--muted-foreground);
}

.offline {
  opacity: 0.85;
}

.offline .status-pill {
  background: color-mix(in srgb, var(--bg-warning) 65%, transparent);
  color: var(--foreground);
}

.error .status-pill {
  background: color-mix(in srgb, var(--destructive) 18%, transparent);
  color: var(--destructive);
}
</style>
