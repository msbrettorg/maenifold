<script setup lang="ts">
import { computed } from 'vue';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import { Button } from '@/components/ui/button';

const props = defineProps<{
  messageId: string;
  thought: string;
  expanded: boolean;
}>();

const emit = defineEmits<{ toggle: [messageId: string] }>();
const renderedThought = computed(() =>
  DOMPurify.sanitize(marked.parse(props.thought, { async: false }) as string)
);
const contentId = computed(() => `thought-content-${props.messageId}`);
</script>

<template>
  <div class="thought-section" role="region" aria-label="Thinking">
    <Button
      variant="ghost"
      class="thought-toggle"
      :aria-expanded="expanded"
      :aria-controls="contentId"
      @click="emit('toggle', messageId)"
    >
      <span class="thought-icon">💭</span>
      <span class="thought-label">{{ expanded ? 'Hide thinking' : 'Show thinking' }}</span>
      <span class="thought-chevron">{{ expanded ? '▲' : '▼' }}</span>
    </Button>
    <div v-if="expanded" :id="contentId" class="thought-content">
      <div v-html="renderedThought" />
    </div>
  </div>
</template>

<style scoped>
.thought-section {
  margin-bottom: 0.75rem;
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: 4px;
}
.thought-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: none;
  background: var(--surface-high);
  color: var(--color-text-secondary);
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
  transition: background 0.15s ease;
}
.thought-toggle:hover { background: var(--surface-highest); }
.thought-icon { flex-shrink: 0; font-size: var(--text-base); }
.thought-label { flex: 1; font-size: var(--text-xs); font-weight: var(--weight-display); }
.thought-chevron { color: var(--color-text-secondary); font-size: var(--text-xs); }
.thought-content {
  padding: 0.75rem 1rem 0.75rem 1.25rem;
  background: var(--surface-container);
  color: var(--color-text-secondary);
  font-size: var(--text-base);
  font-style: italic;
  line-height: 1.5;
}
.thought-content :deep(p) { margin: 0 0 0.5rem; }
.thought-content :deep(p:last-child) { margin-bottom: 0; }
.thought-content :deep(code) {
  padding: 0.125rem 0.25rem;
  border-radius: 4px;
  background: var(--surface-high);
  font-size: var(--text-sm);
}
</style>
