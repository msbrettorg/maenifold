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
  <div class="mb-3 overflow-hidden border border-border-subtle rounded" role="region" aria-label="Thinking">
    <Button
      variant="ghost"
      class="flex items-center gap-2 w-full px-3 py-2 border-none bg-[var(--surface-high)] text-text-secondary text-sm text-left cursor-pointer transition-colors duration-150 hover:bg-[var(--surface-highest)]"
      :aria-expanded="expanded"
      :aria-controls="contentId"
      @click="emit('toggle', messageId)"
    >
      <span class="shrink-0 text-base">💭</span>
      <span class="flex-1 text-xs font-[number:var(--weight-display)]">{{ expanded ? 'Hide thinking' : 'Show thinking' }}</span>
      <span class="text-text-secondary text-xs">{{ expanded ? '▲' : '▼' }}</span>
    </Button>
    <div v-if="expanded" :id="contentId" class="thought-content py-3 pl-5 pr-4 bg-[var(--surface-container)] text-text-secondary text-base italic leading-[1.5]">
      <div v-html="renderedThought" />
    </div>
  </div>
</template>

<!-- audit-ok: max-style-rules — deep-third-party: :deep() rules style sanitized markdown HTML rendered via v-html (p, code, pre, pre code) inside .thought-content; Tailwind utilities cannot reach this child markup across the scoped boundary. -->
<style scoped>
.thought-content :deep(p) { margin: 0 0 0.5rem; }
.thought-content :deep(p:last-child) { margin-bottom: 0; }
.thought-content :deep(code) {
  padding: 0.125rem 0.25rem;
  border-radius: 4px;
  background: var(--surface-high);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  font-style: normal;
}
.thought-content :deep(pre) {
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
  background: var(--color-bg-app);
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}
.thought-content :deep(pre code) {
  background: transparent;
  padding: 0;
  font-size: var(--text-sm);
}
</style>
