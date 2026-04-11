<script setup lang="ts">
import { computed, ref } from 'vue';
import type { TrafficEntry as TrafficLogEntry } from '../stores/traffic';

const props = defineProps<{
  entry: TrafficLogEntry;
}>();

const isExpanded = ref(false);

const entryClass = computed(() => {
  const classes = ['entry', props.entry.direction];
  if (props.entry.error) classes.push('error');
  return classes.join(' ');
});

const directionIcon = computed(() => (props.entry.direction === 'out' ? '→' : '←'));
const typeLabel = computed(() => {
  if (props.entry.type === 'notification') return '(notification)';
  if (props.entry.type === 'response') return '(response)';
  return '';
});

function toggleExpand() {
  isExpanded.value = !isExpanded.value;
}

function formatTime(timestamp: number) {
  const date = new Date(timestamp);
  return `${date.toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })}.${String(date.getMilliseconds()).padStart(3, '0')}`;
}

function formatJson(payload: unknown) {
  try {
    return JSON.stringify(payload, null, 2);
  } catch {
    return String(payload);
  }
}

function copyPayload() {
  navigator.clipboard.writeText(formatJson(props.entry.payload));
}
</script>

<template>
  <div :class="entryClass">
    <div class="entry-header" @click="toggleExpand">
      <span class="expand-icon">{{ isExpanded ? '▾' : '▸' }}</span>
      <span class="timestamp">{{ formatTime(entry.timestamp) }}</span>
      <span class="direction-icon">{{ directionIcon }}</span>
      <span class="method">{{ entry.method }}</span>
      <span class="type-label">{{ typeLabel }}</span>
      <span v-if="entry.requestId !== undefined" class="request-id">#{{ entry.requestId }}</span>
      <button class="copy-btn" title="Copy JSON" @click.stop="copyPayload">📋</button>
    </div>

    <div v-if="isExpanded" class="entry-payload">
      <pre>{{ formatJson(entry.payload) }}</pre>
    </div>
  </div>
</template>

<style scoped>
.entry {
  border-bottom: 1px solid var(--border-default);
}

.entry-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  cursor: pointer;
}

.entry-header:hover {
  background: var(--bg-surface);
}

.expand-icon {
  width: 1em;
  color: var(--text-muted);
}

.timestamp,
.type-label {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.direction-icon {
  width: 1em;
  font-weight: bold;
}

.entry.out .direction-icon {
  color: var(--border-focus);
}

.entry.in .direction-icon {
  color: var(--color-status-completed);
}

.entry.error .direction-icon,
.entry.error .method {
  color: var(--text-error);
}

.method {
  color: var(--text-primary);
  font-weight: 600;
}

.request-id {
  margin-left: auto;
  color: var(--border-focus);
  font-size: var(--text-xs);
}

.copy-btn {
  padding: 0.125rem 0.25rem;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: var(--text-xs);
  opacity: 0;
  transition: opacity 0.15s;
}

.entry-header:hover .copy-btn {
  opacity: 1;
}

.entry-payload {
  padding: 0.5rem 0.75rem 0.75rem 2rem;
  background: var(--bg-code);
  overflow-x: auto;
}

.entry-payload pre {
  margin: 0;
  color: var(--text-code);
  font-size: var(--text-xs);
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
