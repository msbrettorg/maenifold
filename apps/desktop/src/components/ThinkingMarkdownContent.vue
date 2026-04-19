<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';

const props = withDefaults(defineProps<{
  content?: string;
  clamp?: boolean;
}>(), {
  content: '',
  clamp: false,
});

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

function renderMarkdown(content: string): string {
  if (!content) return '';

  const withWikiPills = content.replace(/\[\[([^\]]+)\]\]/g, (_match, label: string) => {
    return `<span class="wiki-pill">${escapeHtml(label)}</span>`;
  });

  const result = marked.parse(withWikiPills, { async: false });
  return DOMPurify.sanitize(typeof result === 'string' ? result : '');
}

const renderedContentWithDOMPurify = computed(() => renderMarkdown(props.content));
</script>

<template>
  <component
    :is="clamp ? 'div' : 'section'"
    class="thinking-markdown-content"
    :class="{ 'thinking-markdown-content--clamped': clamp }"
    v-html="renderedContentWithDOMPurify"
  />
</template>

<style scoped>
.thinking-markdown-content {
  --thinking-markdown-text: color-mix(in srgb, var(--color-text-primary) 84%, var(--color-node-prompt));
  --thinking-markdown-accent: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-node-prompt));
  --thinking-markdown-link: color-mix(in srgb, var(--color-text-primary) 68%, var(--color-status-active));
  --thinking-markdown-border: color-mix(in srgb, var(--color-border-subtle) 32%, var(--color-bg-surface));
  font-size: var(--text-caption);
  line-height: 1.5;
  color: var(--thinking-markdown-text);
  overflow: hidden;
  overflow-wrap: anywhere;
}

.thinking-markdown-content--clamped {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.thinking-markdown-content :deep(p) {
  margin: 0;
}

.thinking-markdown-content :deep(p + p) {
  margin-top: 0.35rem;
}

.thinking-markdown-content :deep(h1),
.thinking-markdown-content :deep(h2),
.thinking-markdown-content :deep(h3),
.thinking-markdown-content :deep(h4),
.thinking-markdown-content :deep(h5),
.thinking-markdown-content :deep(h6) {
  margin: 0.6rem 0 0.35rem;
  color: var(--thinking-markdown-accent);
  font-weight: var(--weight-title);
  line-height: 1.3;
}

.thinking-markdown-content :deep(h1:first-child),
.thinking-markdown-content :deep(h2:first-child),
.thinking-markdown-content :deep(h3:first-child),
.thinking-markdown-content :deep(h4:first-child),
.thinking-markdown-content :deep(h5:first-child),
.thinking-markdown-content :deep(h6:first-child) {
  margin-top: 0;
}

.thinking-markdown-content :deep(ul),
.thinking-markdown-content :deep(ol) {
  margin: 0.2rem 0 0;
  padding-left: 1rem;
}

.thinking-markdown-content :deep(code) {
  background: color-mix(in srgb, var(--color-bg-surface) 100%, var(--color-bg-app));
  color: var(--thinking-markdown-accent);
  border-radius: 4px;
  padding: 0.1rem 0.3rem;
  font-size: var(--text-tiny);
  font-family: var(--font-code);
}

.thinking-markdown-content :deep(pre) {
  background: color-mix(in srgb, var(--color-bg-surface) 100%, var(--color-bg-app));
  border-radius: var(--radius);
  padding: 0.55rem;
  overflow-x: auto;
}

.thinking-markdown-content :deep(pre code) {
  background: transparent;
  padding: 0;
}

.thinking-markdown-content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
  font-size: var(--text-caption);
  overflow-x: auto;
  display: block;
}

.thinking-markdown-content :deep(th),
.thinking-markdown-content :deep(td) {
  border: 1px solid var(--thinking-markdown-border);
  padding: 0.5rem 0.75rem;
  text-align: left;
}

.thinking-markdown-content :deep(th) {
  background: var(--color-bg-surface);
  font-weight: var(--weight-title);
}

.thinking-markdown-content :deep(tr:nth-child(even)) {
  background: var(--color-bg-elevated);
}

.thinking-markdown-content :deep(.wiki-pill) {
  display: inline-flex;
  align-items: center;
  padding: 0.08rem 0.42rem;
  margin: 0 0.08rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-brand) 18%, var(--color-bg-surface));
  color: var(--thinking-markdown-link);
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  white-space: nowrap;
}
</style>
