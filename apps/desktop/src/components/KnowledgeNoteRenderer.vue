<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue';
import DOMPurify from 'dompurify';
import { marked, type TokenizerAndRendererExtension } from 'marked';
import mermaid from 'mermaid';

const props = defineProps<{ content: string }>();
const containerRef = ref<HTMLElement | null>(null);

mermaid.initialize({
  startOnLoad: false,
  theme: 'dark',
  themeVariables: {
    primaryColor: '#0078D4',
    primaryTextColor: '#FFFFFF',
    primaryBorderColor: 'rgba(255,255,255,0.12)',
    lineColor: '#A0AAB2',
    secondaryColor: '#333333',
    tertiaryColor: '#282828',
    fontFamily: "'Segoe UI Variable', 'Segoe UI', system-ui, -apple-system, sans-serif",
    fontSize: '13px',
  },
});

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

const mermaidExtension: TokenizerAndRendererExtension = {
  name: 'mermaidBlock',
  level: 'block',
  start(src: string) { return src.indexOf('```mermaid'); },
  tokenizer(src: string) {
    const match = src.match(/^```mermaid\s*\n([\s\S]*?)```/);
    if (match) {
      return { type: 'mermaidBlock', raw: match[0], text: match[1].trim() };
    }
    return undefined;
  },
  renderer(token) {
    return `<pre class="mermaid-block">${escapeHtml((token as unknown as { text: string }).text)}</pre>`;
  },
};

marked.use({ extensions: [mermaidExtension] });

function renderMarkdown(content: string): string {
  if (!content) return '';
  const withWikiPills = content.replace(/\[\[([^\]]+)\]\]/g, (_match, label: string) => {
    return `<span class="note-wiki-pill">${escapeHtml(label)}</span>`;
  });
  const result = marked.parse(withWikiPills, { async: false });
  return DOMPurify.sanitize(typeof result === 'string' ? result : '');
}

const rendered = computed(() => renderMarkdown(props.content));

async function renderMermaidBlocks() {
  if (!containerRef.value) return;
  const blocks = containerRef.value.querySelectorAll('.mermaid-block');
  if (blocks.length === 0) return;
  for (const block of blocks) {
    const code = block.textContent ?? '';
    const id = `mermaid-${Math.random().toString(36).slice(2, 10)}`;
    try {
      const { svg } = await mermaid.render(id, code);
      block.innerHTML = svg;
      block.classList.add('mermaid-rendered');
    } catch {
      block.classList.add('mermaid-error');
    }
  }
}

watch(rendered, async () => {
  await nextTick();
  renderMermaidBlocks();
});
</script>

<template>
  <article ref="containerRef" class="note-prose" v-html="rendered" />
</template>

<style scoped>
.note-prose {
  font-family: 'Segoe UI Variable', 'Segoe UI', system-ui, -apple-system, sans-serif;
  font-size: 14px;
  line-height: 1.7;
  color: var(--color-text-primary, #FFFFFF);
  overflow-wrap: break-word;
}

/* ── Headings ── */
.note-prose :deep(h1) {
  font-size: 24px;
  font-weight: 600;
  line-height: 1.3;
  margin: 1.75rem 0 0.75rem;
  color: var(--color-text-primary, #FFFFFF);
  border-bottom: 1px solid var(--color-border-subtle, rgba(255,255,255,0.08));
  padding-bottom: 0.5rem;
}
.note-prose :deep(h2) {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.3;
  margin: 1.5rem 0 0.5rem;
  color: var(--color-text-primary, #FFFFFF);
}
.note-prose :deep(h3) {
  font-size: 16px;
  font-weight: 600;
  line-height: 1.4;
  margin: 1.25rem 0 0.4rem;
  color: var(--color-text-primary, #FFFFFF);
}
.note-prose :deep(h4),
.note-prose :deep(h5),
.note-prose :deep(h6) {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.4;
  margin: 1rem 0 0.35rem;
  color: var(--color-text-secondary, #A0AAB2);
}
.note-prose :deep(h1:first-child),
.note-prose :deep(h2:first-child),
.note-prose :deep(h3:first-child) {
  margin-top: 0;
}

/* ── Paragraphs ── */
.note-prose :deep(p) {
  margin: 0 0 0.75rem;
}
.note-prose :deep(p:last-child) {
  margin-bottom: 0;
}

/* ── Links ── */
.note-prose :deep(a) {
  color: var(--color-brand, #0078D4);
  text-decoration: none;
}
.note-prose :deep(a:hover) {
  text-decoration: underline;
}

/* ── Lists ── */
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

/* ── Inline code ── */
.note-prose :deep(code) {
  background: var(--color-bg-surface, #333333);
  color: var(--color-text-primary, #FFFFFF);
  border-radius: 4px;
  padding: 0.15rem 0.4rem;
  font-size: 13px;
  font-family: 'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace;
}

/* ── Code blocks ── */
.note-prose :deep(pre) {
  background: var(--color-bg-surface, #333333);
  border: 1px solid var(--color-border-card, rgba(255,255,255,0.08));
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

/* ── Mermaid ── */
.note-prose :deep(.mermaid-block) {
  background: var(--color-bg-elevated, #2D2D2D);
  border: 1px solid var(--color-border-card, rgba(255,255,255,0.08));
  border-radius: 8px;
  padding: 1.5rem;
  margin: 1rem 0;
  text-align: center;
  overflow-x: auto;
}
.note-prose :deep(.mermaid-rendered) {
  font-family: inherit;
}
.note-prose :deep(.mermaid-error) {
  color: var(--color-danger, #D13438);
  font-size: 13px;
  white-space: pre-wrap;
}

/* ── Tables ── */
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
  border: 1px solid var(--color-border-subtle, rgba(255,255,255,0.08));
  padding: 0.5rem 0.75rem;
  text-align: left;
}
.note-prose :deep(th) {
  background: var(--color-bg-surface, #333333);
  font-weight: 600;
  color: var(--color-text-primary, #FFFFFF);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}
.note-prose :deep(tr:nth-child(even)) {
  background: rgba(255, 255, 255, 0.02);
}

/* ── Blockquotes ── */
.note-prose :deep(blockquote) {
  border-left: 3px solid var(--color-brand, #0078D4);
  padding: 0.5rem 0 0.5rem 1rem;
  margin: 0.75rem 0;
  color: var(--color-text-secondary, #A0AAB2);
  font-style: italic;
}
.note-prose :deep(blockquote p) {
  margin-bottom: 0.25rem;
}

/* ── Horizontal rules ── */
.note-prose :deep(hr) {
  border: none;
  border-top: 1px solid var(--color-border-subtle, rgba(255,255,255,0.08));
  margin: 1.5rem 0;
}

/* ── WikiLink pills ── */
.note-prose :deep(.note-wiki-pill) {
  display: inline-flex;
  align-items: center;
  padding: 0.1rem 0.5rem;
  margin: 0 0.1rem;
  border-radius: 4px;
  background: rgba(0, 120, 212, 0.12);
  color: var(--color-brand, #0078D4);
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  cursor: default;
}

/* ── Strong / em ── */
.note-prose :deep(strong) {
  font-weight: 600;
  color: var(--color-text-primary, #FFFFFF);
}
.note-prose :deep(em) {
  font-style: italic;
}

/* ── Images ── */
.note-prose :deep(img) {
  max-width: 100%;
  border-radius: 8px;
  margin: 0.75rem 0;
}
</style>
