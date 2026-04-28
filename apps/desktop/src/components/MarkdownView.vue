<script setup lang="ts">
import { nextTick, ref, useTemplateRef, watch } from 'vue';
import DOMPurify from 'dompurify';
import { getMarkdownIt } from '../composables/useMarkdownIt';
import { useMermaid } from '../composables/useMermaid';
import { useKnowledgeStore } from '../stores/knowledge';

const props = withDefaults(
  defineProps<{
    content: string;
    clamp?: boolean;
  }>(),
  { clamp: false },
);

const containerRef = useTemplateRef<HTMLElement>('containerRef');
const rendered = ref('');
let renderVersion = 0;
const { renderMermaidBlocks } = useMermaid();
const knowledgeStore = useKnowledgeStore();

async function updateRendered(): Promise<void> {
  const thisVersion = ++renderVersion;
  const source = props.content ?? '';
  if (!source) {
    rendered.value = '';
    return;
  }

  try {
    const md = await getMarkdownIt();
    if (thisVersion !== renderVersion) return;

    const html = md.render(source);
    const safe = DOMPurify.sanitize(html, {
      ADD_ATTR: ['class', 'style', 'data-wikilink', 'data-mermaid-processed', 'disabled', 'checked'],
    });
    if (thisVersion !== renderVersion) return;

    rendered.value = safe;
  } catch {
    if (thisVersion !== renderVersion) return;
    rendered.value = source
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\n/g, '<br>');
    return;
  }

  await nextTick();
  if (thisVersion !== renderVersion) return;
  if (containerRef.value) {
    await renderMermaidBlocks(containerRef.value);
  }
}

function handleClick(event: MouseEvent): void {
  if (!(event.target instanceof Element)) return;

  const target = event.target.closest<HTMLElement>('[data-wikilink]');
  if (!target) return;

  const concept = target.getAttribute('data-wikilink');
  if (!concept) return;

  event.preventDefault();
  const uri = `memory://${concept}`;
  if (event.metaKey || event.ctrlKey) {
    knowledgeStore.openInNewTab(uri);
    return;
  }
  knowledgeStore.openNote(uri);
}

watch(() => props.content, updateRendered, { immediate: true });
</script>

<template>
  <article
    ref="containerRef"
    class="prose prose-sm min-w-0 max-w-full [overflow-wrap:anywhere] dark:prose-invert [&_pre]:max-w-full [&_pre]:overflow-x-auto [&_pre]:whitespace-pre [&_pre_code]:whitespace-pre [&_table]:block [&_table]:max-w-full [&_table]:overflow-x-auto"
    :class="clamp ? 'line-clamp-6' : ''"
    @click="handleClick"
    v-html="rendered"
  />
</template>
