<script setup lang="ts">
import { computed, ref } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { Button } from '@/components/ui/button';

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
  <section class="rounded-md bg-[var(--card,var(--color-bg-app))] border border-border-subtle">
    <Button
      variant="ghost"
      class="w-full flex items-center justify-between gap-3 py-3 px-3.5 rounded-md bg-transparent text-text-primary font-[var(--weight-title)] hover:bg-[color-mix(in_srgb,var(--card,var(--color-bg-app))_88%,white_12%)] h-auto"
      :aria-expanded="isExpanded"
      @click="isExpanded = !isExpanded"
    >
      <span>Assembled Prompt</span>
      <span class="text-text-secondary text-sm" aria-hidden="true">{{ isExpanded ? '▾' : '▸' }}</span>
    </Button>

    <div v-if="isExpanded" class="flex flex-col gap-3 px-3.5 pb-3.5">
      <div
        v-for="section in sections"
        :key="section.key"
        class="flex flex-col gap-1.5"
      >
        <div class="text-xs font-[var(--weight-title)] tracking-[0.02em] text-text-secondary">{{ section.label }}</div>
        <div
          class="assembly-preview__content m-0 max-h-48 overflow-auto p-3 rounded-md border border-border-subtle bg-bg-app text-text-primary font-mono text-sm leading-normal [overflow-wrap:anywhere]"
          v-html="renderMarkdown(section.content)"
        />
      </div>
    </div>
  </section>
</template>

<!-- audit-ok: max-style-rules deep-third-party: :deep() rules style sanitized markdown HTML rendered via v-html inside .assembly-preview__content; Tailwind utilities cannot reach this child markup across the scoped boundary. -->
<style scoped>
.assembly-preview__content :deep(*) {
  margin-top: 0;
}

.assembly-preview__content :deep(p:last-child) {
  margin-bottom: 0;
}

.assembly-preview__content :deep(code) {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  background: var(--color-bg-app);
  border-radius: var(--radius);
  padding: 0.1rem 0.3rem;
}

.assembly-preview__content :deep(pre) {
  margin: 0.75rem 0 0;
  padding: 0.75rem;
  background: var(--color-bg-app);
  border-radius: var(--radius);
  overflow-x: auto;
}
</style>
