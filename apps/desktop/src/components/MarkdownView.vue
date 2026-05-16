<script setup lang="ts">
import { nextTick, ref, useTemplateRef, watch } from 'vue';
import { getMarkdownIt } from '../composables/useMarkdownIt';
import { useMemoryApi } from '../composables/useMemoryApi';
import { useMermaid } from '../composables/useMermaid';
import { ToolEnvelopeError } from '../composables/pipeline-mcp-transport';
import { collectFolderPaths } from '../lib/knowledge-helpers';
import { ALLOWED_URI_REGEXP, sanitize } from '../lib/sanitizer';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import WikilinkCreateDialog from './WikilinkCreateDialog.vue';

const props = withDefaults(
  defineProps<{
    content: string;
    clamp?: boolean;
    wikilinkAction?: 'open-note' | 'search-knowledge';
  }>(),
  { clamp: false, wikilinkAction: 'open-note' },
);

const containerRef = useTemplateRef<HTMLElement>('containerRef');
const rendered = ref('');
let renderVersion = 0;
const { renderMermaidBlocks } = useMermaid();
const appShellStore = useAppShellStore();
const knowledgeStore = useKnowledgeStore();
const memoryApi = useMemoryApi();
const createDialogOpen = ref(false), unresolvedConcept = ref('');
const wikilinkExistsCache = new Map<string, boolean>();
let unresolvedCheckTimer: number | null = null;

const forbiddenTags = ['script', 'iframe', 'object', 'embed', 'form', 'input', 'button', 'textarea'];

function isNotFoundError(error: unknown): boolean {
  return error instanceof ToolEnvelopeError && error.code === 'NOT_FOUND';
}

async function conceptExists(concept: string): Promise<boolean | null> { const cached = wikilinkExistsCache.get(concept); if (cached !== undefined) return cached; try { await memoryApi.readMemory(`memory://${concept}`); wikilinkExistsCache.set(concept, true); return true; } catch (error) { if (isNotFoundError(error)) { wikilinkExistsCache.set(concept, false); return false; } return null; } }

async function runWithLimit<T>(items: T[], limit: number, fn: (item: T) => Promise<void>): Promise<void> {
  let nextIndex = 0;
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (nextIndex < items.length) {
      const item = items[nextIndex];
      nextIndex += 1;
      await fn(item);
    }
  });
  await Promise.all(workers);
}

async function markUnresolvedWikilinks(): Promise<void> { const chips = Array.from(containerRef.value?.querySelectorAll<HTMLElement>('[data-wikilink]') ?? []); await runWithLimit(chips, 4, async (chip) => { const concept = chip.getAttribute('data-wikilink'); if (!concept) return; const exists = await conceptExists(concept); if (exists === false) chip.setAttribute('data-unresolved', 'true'); else chip.removeAttribute('data-unresolved'); }); }

function scheduleUnresolvedCheck(): void { if (unresolvedCheckTimer !== null) window.clearTimeout(unresolvedCheckTimer); unresolvedCheckTimer = window.setTimeout(() => { void markUnresolvedWikilinks(); }, 60); }

function restoreForeignObjectText(safeHtml: string, originalHtml: string): string {
  if (!originalHtml.includes('<foreignObject') && !originalHtml.includes('<foreignobject')) {
    return safeHtml;
  }

  const parser = new DOMParser();
  const originalDocument = parser.parseFromString(`<body>${originalHtml}</body>`, 'text/html');
  const safeDocument = parser.parseFromString(`<body>${safeHtml}</body>`, 'text/html');
  const originalForeignObjects = Array.from(originalDocument.body.querySelectorAll('foreignObject, foreignobject'));
  const safeForeignObjects = Array.from(safeDocument.body.querySelectorAll('foreignObject, foreignobject'));

  safeForeignObjects.forEach((safeNode, index) => {
    const originalNode = originalForeignObjects[index];
    if (!originalNode) return;

    const sanitizedInnerHtml = sanitize(originalNode.innerHTML, {
      USE_PROFILES: { html: true },
      FORBID_TAGS: [...forbiddenTags, 'style'],
      ALLOWED_URI_REGEXP,
    });
    const container = document.createElement('div');
    container.innerHTML = sanitizedInnerHtml;
    const text = container.textContent ?? '';
    if (text) safeNode.textContent = text;
  });

  return safeDocument.body.innerHTML;
}

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
    const safe = sanitize(html, {
      USE_PROFILES: { html: true, svg: true, svgFilters: true },
      ADD_TAGS: ['style', 'foreignObject', 'use'],
      ADD_ATTR: [
        'class',
        'style',
        'data-wikilink',
        'data-mermaid-processed',
        'disabled',
        'checked',
        'xmlns',
        'viewBox',
        'fill',
        'stroke',
        'd',
        'cx',
        'cy',
        'r',
        'width',
        'height',
        'x',
        'y',
        'rx',
        'ry',
        'transform',
        'points',
        'x1',
        'y1',
        'x2',
        'y2',
        'id',
        'href',
        'xlink:href',
        'xmlns:xlink',
      ],
      FORBID_TAGS: forbiddenTags,
      ALLOWED_URI_REGEXP,
    });
    if (thisVersion !== renderVersion) return;

    rendered.value = restoreForeignObjectText(safe, html);
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
    scheduleUnresolvedCheck();
  }
}

async function handleClick(event: MouseEvent): Promise<void> {
  if (!(event.target instanceof Element)) return;

  const linkTarget = event.target.closest<HTMLAnchorElement>('a');
  const href = linkTarget?.getAttribute('href') ?? '';
  if (href.startsWith('memory://')) {
    event.preventDefault();
    if (href === 'memory://' || href === 'memory:///') return;
    if (event.metaKey || event.ctrlKey) knowledgeStore.openInNewTab(href);
    else knowledgeStore.openNote(href);
    appShellStore.setActiveTab('knowledge');
    return;
  }

  const target = event.target.closest<HTMLElement>('[data-wikilink]');
  if (!target) return;

  const concept = target.getAttribute('data-wikilink');
  if (!concept) return;

  event.preventDefault();
  if (props.wikilinkAction === 'search-knowledge') {
    knowledgeStore.setSearchQuery(concept);
    appShellStore.setActiveTab('knowledge');
    return;
  }

  const uri = `memory://${concept}`;
  const exists = await conceptExists(concept);
  if (exists === false) {
    unresolvedConcept.value = concept; createDialogOpen.value = true;
    return;
  }
  if (exists === null) return;
  if (event.metaKey || event.ctrlKey) { knowledgeStore.openInNewTab(uri); return; }
  knowledgeStore.openNote(uri);
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key !== ' ') return;
  if (!(event.target instanceof Element)) return;
  const target = event.target.closest<HTMLElement>('[data-wikilink]');
  if (!target) return;
  event.preventDefault();
  target.click();
}

function handleCreated(): void {
  wikilinkExistsCache.set(unresolvedConcept.value, true); void knowledgeStore.loadRootFolders(); void markUnresolvedWikilinks();
} watch(() => props.content, updateRendered, { immediate: true });

defineExpose({ markUnresolvedWikilinks, wikilinkExistsCache });
</script>

<template>
  <article
    ref="containerRef"
    class="prose prose-sm min-w-0 max-w-full [overflow-wrap:anywhere] dark:prose-invert [&_a[data-wikilink][data-unresolved=true]]:border-dashed [&_a[data-wikilink][data-unresolved=true]]:opacity-70 [&_pre]:max-w-full [&_pre]:overflow-x-auto [&_pre]:whitespace-pre [&_pre_code]:whitespace-pre [&_table]:block [&_table]:max-w-full [&_table]:overflow-x-auto"
    :class="clamp ? 'line-clamp-6' : ''"
    @click="handleClick"
    @keydown="handleKeydown"
    v-html="rendered"
  />
  <WikilinkCreateDialog v-model:open="createDialogOpen" :concept="unresolvedConcept" :available-folders="collectFolderPaths(knowledgeStore.folders)" @created="handleCreated" />
</template>
