<script setup lang="ts">
import { computed, ref } from 'vue';
import TrafficJsonPayload from './TrafficJsonPayload.vue';
import { Button } from '@/components/ui/button';
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

const barClass = computed(() => {
  if (props.entry.error) return 'before:bg-danger';
  if (props.entry.direction === 'out') return 'before:bg-brand';
  if (props.entry.direction === 'in') return 'before:bg-thinking';
  return 'before:bg-text-secondary';
});

const directionIconClass = computed(() => {
  if (props.entry.error) return 'text-danger';
  if (props.entry.direction === 'out') return 'text-brand';
  return 'text-text-secondary';
});

const methodClass = computed(() => (props.entry.error ? 'text-danger' : 'text-text-primary'));

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
    :class="[
      'relative odd:bg-surface even:bg-surface-container',
      'before:content-[\'\'] before:absolute before:inset-y-0 before:left-0 before:w-1',
      barClass,
    ]"
    role="article"
    :aria-label="entryAriaLabel"
  >
    <div
      class="group flex items-center gap-2 py-1.5 pr-3 pl-4 cursor-pointer hover:bg-surface-high focus-visible:outline-2 focus-visible:outline-brand focus-visible:-outline-offset-2"
      role="button"
      tabindex="0"
      @click="toggleExpand"
      @keydown.enter="toggleExpand"
      @keydown.space.prevent="toggleExpand"
    >
      <span class="w-[1em] text-text-secondary">{{ isExpanded ? '▾' : '▸' }}</span>
      <span class="text-text-secondary text-tiny font-mono">{{ formatTime(entry.timestamp) }}</span>
      <span :class="['w-[1em] font-bold', directionIconClass]">{{ directionIcon }}</span>
      <span :class="['font-medium', methodClass]">{{ entry.method }}</span>
      <span v-if="typeLabel" class="text-text-secondary text-tiny">{{ typeLabel }}</span>
      <span v-if="entry.requestId !== undefined" class="ml-auto text-brand text-tiny font-mono">#{{ entry.requestId }}</span>
      <Button
        variant="ghost"
        size="icon-xs"
        class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
        aria-label="Copy JSON"
        @keydown.stop
        @click.stop="copyPayload"
      >📋</Button>
    </div>

    <TrafficJsonPayload v-if="isExpanded" :payload="entry.payload" />
  </article>
</template>
