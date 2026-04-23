<script setup lang="ts">
import { ref, watch, useTemplateRef, nextTick } from 'vue';
import DOMPurify from 'dompurify';
import { getMarkdownIt } from '../composables/useMarkdownIt';
import { useMermaid } from '../composables/useMermaid';

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

async function updateRendered(): Promise<void> {
  const thisVersion = ++renderVersion;
  const source = props.content ?? '';
  if (!source) {
    rendered.value = '';
    return;
  }

  const md = await getMarkdownIt();
  if (thisVersion !== renderVersion) return;

  // markdown-it handles WikiLinks via the wikilinkPlugin (see useMarkdownIt).
  // We do NOT pre-inject HTML here; html:false would escape it.
  const html = md.render(source);
  // DOMPurify sanitization is mandatory for any v-html assignment.
  const safe = DOMPurify.sanitize(html, {
    ADD_ATTR: ['class', 'style'],
  });
  if (thisVersion !== renderVersion) return;

  rendered.value = safe;

  await nextTick();
  if (thisVersion !== renderVersion) return;
  if (containerRef.value) {
    await renderMermaidBlocks(containerRef.value);
    // Re-check after the (async) mermaid pass: if the content changed while
    // we were rendering diagrams, drop our DOM mutations on the floor — Vue
    // will reconcile to the newer `rendered.value` on the next tick anyway.
    if (thisVersion !== renderVersion) return;
  }
}

watch(() => props.content, updateRendered, { immediate: true });
</script>

<template>
  <article
    ref="containerRef"
    class="note-prose"
    :class="{ 'note-prose--clamp': clamp }"
    v-html="rendered"
  />
</template>

<!--
  Styles below target sanitized markdown output (h1-h6, p, code, pre, table,
  blockquote, .mermaid-rendered, .note-wiki-pill) generated from v-html.
  These cannot be expressed via Tailwind utility classes on Vue templates
  because the elements are produced by markdown-it at runtime, not authored
  by hand. @tailwindcss/typography ("prose") is not installed in this app,
  so the prose styles are inlined here using existing design tokens.
-->
<!-- audit-ok: max-style-rules -->
<style scoped>
.note-prose {
  font-family: var(--font-body);
  font-size: 14px;
  line-height: 1.7;
  color: var(--color-text-primary);
  overflow-wrap: break-word;
}
.note-prose--clamp {
  font-size: var(--text-caption);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  overflow: hidden;
}
.note-prose :deep(h1) {
  font-size: 24px;
  font-weight: 600;
  line-height: 1.3;
  margin: 1.75rem 0 0.75rem;
  color: var(--color-text-primary);
  border-bottom: 1px solid var(--color-border-subtle);
  padding-bottom: 0.5rem;
}
.note-prose :deep(h2) {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.3;
  margin: 1.5rem 0 0.5rem;
  color: var(--color-text-primary);
}
.note-prose :deep(h3) {
  font-size: 16px;
  font-weight: 600;
  line-height: 1.4;
  margin: 1.25rem 0 0.4rem;
  color: var(--color-text-primary);
}
.note-prose :deep(h4),
.note-prose :deep(h5),
.note-prose :deep(h6) {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.4;
  margin: 1rem 0 0.35rem;
  color: var(--color-text-secondary);
}
.note-prose :deep(h1:first-child),
.note-prose :deep(h2:first-child),
.note-prose :deep(h3:first-child) {
  margin-top: 0;
}
.note-prose :deep(p) {
  margin: 0 0 0.75rem;
}
.note-prose :deep(p:last-child) {
  margin-bottom: 0;
}
.note-prose :deep(strong) {
  font-weight: 600;
  color: var(--color-text-primary);
}
.note-prose :deep(em) {
  font-style: italic;
}
.note-prose :deep(a) {
  color: var(--color-text-primary);
  text-decoration: underline;
  text-decoration-color: var(--color-brand);
  text-underline-offset: 2px;
}
.note-prose :deep(a:hover) {
  text-decoration-color: var(--color-text-primary);
}
.note-prose :deep(ul),
.note-prose :deep(ol) {
  margin: 0 0 0.75rem;
  padding-left: 1.5rem;
}
.note-prose :deep(li) {
  margin-bottom: 0.25rem;
}
.note-prose :deep(li > ul),
.note-prose :deep(li > ol) {
  margin-top: 0.25rem;
  margin-bottom: 0;
}
.note-prose :deep(code) {
  background: var(--color-bg-surface);
  color: var(--color-text-primary);
  border-radius: 4px;
  padding: 0.15rem 0.4rem;
  font-size: 13px;
  font-family: var(--font-code);
}
.note-prose :deep(pre) {
  background: var(--color-bg-surface);
  border: 1px solid var(--color-border-card);
  border-radius: 8px;
  padding: 1rem;
  margin: 0.75rem 0;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.5;
}
.note-prose :deep(pre code) {
  background: transparent;
  padding: 0;
  border-radius: 0;
  font-size: inherit;
}
.note-prose :deep(.shiki) {
  background: var(--color-bg-surface) !important;
}
.note-prose :deep(.mermaid-rendered) {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-card);
  border-radius: 8px;
  padding: 1.5rem;
  margin: 1rem 0;
  text-align: center;
  overflow-x: auto;
  font-family: inherit;
}
.note-prose :deep(.mermaid-error) {
  color: var(--color-status-stuck);
  font-size: 13px;
  white-space: pre-wrap;
}
.note-prose :deep(.mermaid-error-message) {
  color: var(--color-status-stuck);
  font-size: 13px;
  margin-top: 0.5rem;
}
.note-prose :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
  font-size: 13px;
  display: block;
  overflow-x: auto;
}
.note-prose :deep(th),
.note-prose :deep(td) {
  border: 1px solid var(--color-border-subtle);
  padding: 0.5rem 0.75rem;
  text-align: left;
}
.note-prose :deep(th) {
  background: var(--color-bg-surface);
  font-weight: 600;
  color: var(--color-text-primary);
  font-size: 12px;
  letter-spacing: 0.03em;
}
.note-prose :deep(tr:nth-child(even)) {
  background: color-mix(in srgb, var(--color-bg-elevated) 35%, transparent);
}
.note-prose :deep(blockquote) {
  border-left: 3px solid var(--color-brand);
  padding: 0.5rem 0 0.5rem 1rem;
  margin: 0.75rem 0;
  color: var(--color-text-secondary);
  font-style: italic;
}
.note-prose :deep(blockquote p) {
  margin-bottom: 0.25rem;
}
.note-prose :deep(hr) {
  border: none;
  border-top: 1px solid var(--color-border-subtle);
  margin: 1.5rem 0;
}
.note-prose :deep(img) {
  max-width: 100%;
  border-radius: 8px;
  margin: 0.75rem 0;
}
.note-prose :deep(.note-wiki-pill) {
  display: inline-flex;
  align-items: center;
  padding: 0.1rem 0.5rem;
  margin: 0 0.1rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-brand) 22%, transparent);
  color: var(--color-text-primary);
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  cursor: default;
}
.note-prose--clamp :deep(p) {
  margin: 0;
}
.note-prose--clamp :deep(p + p) {
  margin-top: 0.35rem;
}
</style>
