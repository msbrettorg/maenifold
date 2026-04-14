<script setup lang="ts">
import { computed } from 'vue';
import { marked } from 'marked';

const props = defineProps<{
  messageId: string;
  thought: string;
  expanded: boolean;
}>();

const emit = defineEmits<{ toggle: [messageId: string] }>();
const renderedThought = computed(() => marked.parse(props.thought, { async: false }) as string);
</script>

<template>
  <div class="thought-section">
    <button type="button" class="thought-toggle" @click="emit('toggle', messageId)">
      <span class="thought-icon">💭</span>
      <span class="thought-label">{{ expanded ? 'Hide Thinking' : 'Show Thinking' }}</span>
      <span class="thought-chevron">{{ expanded ? '▲' : '▼' }}</span>
    </button>
    <div v-if="expanded" class="thought-content">
      <div v-html="renderedThought" />
    </div>
  </div>
</template>

<style scoped>
.thought-section {
  margin-bottom: 0.75rem;
  overflow: hidden;
  border: 1px solid var(--border-default);
  border-radius: 8px;
}
.thought-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: none;
  background: var(--bg-surface);
  color: var(--text-muted);
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
  transition: background 0.15s ease;
}
.thought-toggle:hover { background: color-mix(in srgb, var(--border-focus) 24%, var(--bg-panel)); }
.thought-icon { flex-shrink: 0; font-size: var(--text-base); }
.thought-label { flex: 1; font-size: var(--text-xs); font-weight: var(--weight-display); }
.thought-chevron { color: var(--text-muted); font-size: var(--text-xs); }
.thought-content {
  padding: 0.75rem 1rem 0.75rem 1.25rem;
  border-top: 1px solid var(--border-default);
  background: var(--bg-main);
  color: var(--text-muted);
  font-size: var(--text-base);
  font-style: italic;
  line-height: 1.5;
}
.thought-content :deep(p) { margin: 0 0 0.5rem; }
.thought-content :deep(p:last-child) { margin-bottom: 0; }
.thought-content :deep(code) {
  padding: 0.125rem 0.25rem;
  border-radius: 3px;
  background: var(--bg-surface);
  font-size: var(--text-sm);
}
</style>
