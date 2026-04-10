<script setup lang="ts">
import { computed } from 'vue';
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
  return typeof result === 'string' ? result : '';
}

const renderedContent = computed(() => renderMarkdown(props.content));
</script>

<template>
  <div
    class="thinking-markdown-content"
    :class="{ 'thinking-markdown-content--clamped': clamp }"
    v-html="renderedContent"
  />
</template>

<style scoped>
.thinking-markdown-content {
  --thinking-markdown-text: color-mix(in srgb, var(--text-on-accent) 84%, var(--color-node-prompt));
  --thinking-markdown-accent: color-mix(in srgb, var(--text-on-accent) 72%, var(--color-node-prompt));
  --thinking-markdown-link: color-mix(in srgb, var(--text-on-accent) 68%, var(--color-status-active));
  --thinking-markdown-border: color-mix(in srgb, var(--border-default) 32%, var(--bg-code));
  font-size: var(--text-sm);
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

.thinking-markdown-content :deep(ul),
.thinking-markdown-content :deep(ol) {
  margin: 0.2rem 0 0;
  padding-left: 1rem;
}

.thinking-markdown-content :deep(code) {
  background: color-mix(in srgb, var(--bg-code) 100%, var(--bg-main));
  color: var(--thinking-markdown-accent);
  border-radius: 4px;
  padding: 0.1rem 0.3rem;
  font-size: var(--text-xs);
  font-family: var(--font-mono);
}

.thinking-markdown-content :deep(pre) {
  background: color-mix(in srgb, var(--bg-code) 100%, var(--bg-main));
  border-radius: 6px;
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
  font-size: var(--text-sm);
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
  background: var(--bg-surface);
  font-weight: 600;
}

.thinking-markdown-content :deep(tr:nth-child(even)) {
  background: var(--bg-panel);
}

.thinking-markdown-content :deep(.wiki-pill) {
  display: inline-flex;
  align-items: center;
  padding: 0.08rem 0.42rem;
  margin: 0 0.08rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-primary) 18%, var(--bg-code));
  color: var(--thinking-markdown-link);
  font-size: var(--text-xs);
  font-weight: 600;
  white-space: nowrap;
}
</style>
