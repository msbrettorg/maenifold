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

// Keep data URI rendering to inert raster image formats. SVG data URIs are
// intentionally excluded because nested SVG payloads can carry executable
// script/event handlers that would bypass DOM-level SVG sanitization.
const allowedUriRegexp =
  /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$)|data:image\/(?:bmp|gif|jpe?g|png|webp);base64,)/i;
const forbiddenTags = ['script', 'iframe', 'object', 'embed', 'form', 'input', 'button', 'textarea'];
const eventAttributeHookInstalledKey = '__maenifoldEventAttributeHookInstalled';

function installEventAttributeHook(): void {
  const purifierState = DOMPurify as unknown as Record<string, boolean>;
  if (purifierState[eventAttributeHookInstalledKey]) return;
  DOMPurify.addHook('uponSanitizeAttribute', (_node, data) => {
    if (data.attrName?.toLowerCase().startsWith('on')) {
      data.keepAttr = false;
    }
  });
  purifierState[eventAttributeHookInstalledKey] = true;
}

installEventAttributeHook();

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

    const sanitizedInnerHtml = DOMPurify.sanitize(originalNode.innerHTML, {
      USE_PROFILES: { html: true },
      FORBID_TAGS: [...forbiddenTags, 'style'],
      ALLOWED_URI_REGEXP: allowedUriRegexp,
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
    const safe = DOMPurify.sanitize(html, {
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
      ALLOWED_URI_REGEXP: allowedUriRegexp,
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
