<script setup lang="ts">
import { computed, ref } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';

interface PreviewSection {
  key: string;
  label: string;
  content: string;
}

const props = defineProps<{
  assembledPrompt: string;
}>();

const isExpanded = ref(false);

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

const sections = computed<PreviewSection[]>(() => {
  const trimmedPrompt = props.assembledPrompt.trim();
  if (!trimmedPrompt) {
    return [];
  }

  const chunks = trimmedPrompt.split(/\n{2,}/).filter((chunk) => chunk.trim().length > 0);
  if (chunks.length < 2) {
    return [{ key: 'full', label: 'Full Prompt', content: trimmedPrompt }];
  }

  const predecessorChunks: string[] = [];
  let index = 0;

  while (index < chunks.length - 2 && chunks[index].startsWith('### From ')) {
    predecessorChunks.push(chunks[index], chunks[index + 1]);
    index += 2;
  }

  const remainder = chunks.slice(index);
  const parsedSections: PreviewSection[] = [];

  if (predecessorChunks.length > 0) {
    parsedSections.push({
      key: 'predecessor',
      label: 'Predecessor Context',
      content: predecessorChunks.join('\n\n'),
    });
  }

  if (remainder.length > 1) {
    parsedSections.push({
      key: 'ambient',
      label: 'Ambient Prompt',
      content: remainder.slice(0, -1).join('\n\n'),
    });
    parsedSections.push({
      key: 'instruction',
      label: 'Node Instruction',
      content: remainder[remainder.length - 1],
    });
  } else if (remainder.length === 1) {
    parsedSections.push({
      key: predecessorChunks.length > 0 ? 'instruction' : 'full',
      label: predecessorChunks.length > 0 ? 'Node Instruction' : 'Full Prompt',
      content: remainder[0],
    });
  }

  return parsedSections.filter((section) => section.content.trim().length > 0);
});
</script>

<template>
  <section class="assembly-preview">
    <button
      class="assembly-preview__toggle"
      type="button"
      :aria-expanded="isExpanded"
      @click="isExpanded = !isExpanded"
    >
      <span>Assembled Prompt</span>
      <span class="assembly-preview__chevron" aria-hidden="true">{{ isExpanded ? '▾' : '▸' }}</span>
    </button>

    <div v-if="isExpanded" class="assembly-preview__body">
      <div
        v-for="section in sections"
        :key="section.key"
        class="assembly-preview__section"
      >
        <div class="assembly-preview__label">{{ section.label }}</div>
        <div class="assembly-preview__content" v-html="renderMarkdown(section.content)" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.assembly-preview {
  border: 1px solid var(--border-default, var(--border-default));
  border-radius: 10px;
  background: var(--bg-panel, var(--bg-main));
}

.assembly-preview__toggle {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem 0.875rem;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  font-weight: var(--weight-title);
  cursor: pointer;
}

.assembly-preview__toggle:hover {
  background: color-mix(in srgb, var(--bg-panel, var(--bg-main)) 88%, white 12%);
}

.assembly-preview__body {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0 0.875rem 0.875rem;
}

.assembly-preview__section {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.assembly-preview__label {
  font-size: var(--text-xs);
  font-weight: var(--weight-title);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.assembly-preview__content {
  margin: 0;
  max-height: 12rem;
  overflow: auto;
  padding: 0.75rem;
  border-radius: 8px;
  border: 1px solid var(--border-default, var(--border-default));
  background: var(--bg-main);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.assembly-preview__content :deep(*) {
  margin-top: 0;
}

.assembly-preview__content :deep(p:last-child) {
  margin-bottom: 0;
}

.assembly-preview__content :deep(code) {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  background: var(--bg-main);
  border-radius: 6px;
  padding: 0.1rem 0.3rem;
}

.assembly-preview__content :deep(pre) {
  margin: 0.75rem 0 0;
  padding: 0.75rem;
  background: var(--bg-main);
  border-radius: 8px;
  overflow-x: auto;
}

.assembly-preview__chevron {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}
</style>
