<script setup lang="ts">
import { computed, ref } from 'vue';
import TrafficJsonPayload from './TrafficJsonPayload.vue';
import type { TrafficEntry as TrafficLogEntry } from '../stores/traffic';

const props = defineProps<{ entry: TrafficLogEntry }>();

const isExpanded = ref(false);

const directionIcon = computed(() => (props.entry.direction === 'out' ? '→' : '←'));
const entryAriaLabel = computed(() => `${props.entry.method} at ${formatTime(props.entry.timestamp)}`);
const typeLabel = computed(() => {
  if (props.entry.type === 'notification') return 'notification';
  if (props.entry.type === 'response') return 'response';
  return '';
});

function toggleExpand() {
  isExpanded.value = !isExpanded.value;
}

function formatTime(timestamp: number) {
  const d = new Date(timestamp);
  return `${d.toLocaleTimeString('en-US', {
    hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit',
  })}.${String(d.getMilliseconds()).padStart(3, '0')}`;
}

function copyPayload() {
  try {
    navigator.clipboard.writeText(JSON.stringify(props.entry.payload, null, 2));
  } catch { /* clipboard not available */ }
}
</script>

<template>
  <article
    :class="['entry', entry.direction, { error: entry.error }]"
    role="article"
    :aria-label="entryAriaLabel"
  >
    <div class="entry-header" @click="toggleExpand">
      <span class="expand-icon">{{ isExpanded ? '▾' : '▸' }}</span>
      <span class="timestamp">{{ formatTime(entry.timestamp) }}</span>
      <span :class="['direction-icon', entry.direction]">{{ directionIcon }}</span>
      <span class="method">{{ entry.method }}</span>
      <span v-if="typeLabel" class="type-label">{{ typeLabel }}</span>
      <span v-if="entry.requestId !== undefined" class="request-id">#{{ entry.requestId }}</span>
      <button class="copy-btn" title="Copy JSON" @click.stop="copyPayload">📋</button>
    </div>

    <TrafficJsonPayload v-if="isExpanded" :payload="entry.payload" />
  </article>
</template>

<style scoped>
.entry:nth-child(odd) { background: #1c1b1b; }
.entry:nth-child(even) { background: #201f1f; }

.entry-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  cursor: pointer;
}

.entry-header:hover { background: #2a2a2a; }

.expand-icon {
  width: 1em;
  color: #707070;
}

.timestamp {
  color: #707070;
  font-size: var(--text-tiny);
  font-family: var(--font-code);
}

.direction-icon {
  width: 1em;
  font-weight: bold;
}

.direction-icon.out { color: #4d8eff; }
.direction-icon.in { color: #707070; }

.entry.error .direction-icon,
.entry.error .method { color: #ffb4ab; }

.type-label {
  color: #707070;
  font-size: var(--text-tiny);
}

.method {
  color: #e5e2e1;
  font-weight: 500;
}

.request-id {
  margin-left: auto;
  color: #4d8eff;
  font-size: var(--text-tiny);
  font-family: var(--font-code);
}

.copy-btn {
  padding: 0.125rem 0.25rem;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: var(--text-tiny);
  opacity: 0;
  transition: opacity 0.15s;
}

.entry-header:hover .copy-btn { opacity: 1; }
</style>
