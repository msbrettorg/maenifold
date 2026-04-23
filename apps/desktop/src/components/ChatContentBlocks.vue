<script setup lang="ts">
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import type { ContentBlock } from '../lib/types';

defineProps<{ blocks: ContentBlock[] }>();

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  const html = DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
  // Guard: if markdown consumed the input into empty HTML structures
  // (e.g. "\n\n4." → <ol><li></li></ol>), fall back to escaped plaintext.
  const visibleText = html.replace(/<[^>]*>/g, '').trim();
  if (visibleText.length === 0 && content.trim().length > 0) {
    const escaped = content
      .trim()
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    return `<p>${escaped}</p>`;
  }
  return html;
}

function getImageSrc(block: ContentBlock): string {
  return block.type === 'image' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getAudioSrc(block: ContentBlock): string {
  return block.type === 'audio' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getResourceLinkLabel(block: ContentBlock): string {
  return block.type === 'resource_link' ? block.title || block.name || block.uri : '';
}

function getResourceLinkDescription(block: ContentBlock): string | null {
  if (block.type !== 'resource_link' || typeof block.description !== 'string') return null;
  const trimmed = block.description.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function formatFileSize(bytes: number | bigint): string {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
  if (typeof bytes === 'bigint') {
    if (bytes < 1024n) return `${bytes} B`;
    let value = bytes;
    let unitIndex = 0;
    while (value >= 1024n && unitIndex < units.length - 1) {
      value /= 1024n;
      unitIndex += 1;
    }
    return `${value} ${units[unitIndex]}`;
  }
  if (!Number.isFinite(bytes) || bytes < 0) return '';
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unitIndex]}`;
}

function getResourceLinkSizeLabel(block: ContentBlock): string | null {
  return block.type === 'resource_link' && block.size != null ? formatFileSize(block.size) : null;
}

function getResourceLinkTooltip(block: ContentBlock): string | null {
  if (block.type !== 'resource_link') return null;
  const parts = [getResourceLinkDescription(block), getResourceLinkSizeLabel(block)].filter(Boolean);
  return parts.length ? parts.join(' • ') : null;
}

function getResourceUri(block: ContentBlock): string {
  return block.type === 'resource' ? block.resource.uri : '';
}

function getResourceBody(block: ContentBlock): string {
  if (block.type !== 'resource') return '';
  return 'text' in block.resource ? block.resource.text : block.resource.blob;
}

function getBlockAriaLabel(block: ContentBlock): string {
  switch (block.type) {
    case 'text':
      return 'Text content';
    case 'image':
      return 'Image content';
    case 'audio':
      return 'Audio content';
    case 'resource_link':
      return 'Resource link';
    case 'resource':
      return 'Resource content';
    default:
      return 'Content block';
  }
}
</script>

<template>
  <div
    v-for="(block, index) in blocks"
    :key="`${block.type}-${index}`"
    class="message-content-block [&+&]:mt-3"
    role="region"
    :aria-label="getBlockAriaLabel(block)"
  >
    <div v-if="block.type === 'text'" class="message-content leading-relaxed [overflow-wrap:break-word] [word-wrap:break-word]" v-html="renderMarkdown(block.text)" />
    <img
      v-else-if="block.type === 'image'"
      class="block max-w-[min(100%,32rem)] max-h-96 border border-border-subtle rounded"
      :src="getImageSrc(block)"
      :alt="block.uri ?? 'Message image'"
    />
    <audio
      v-else-if="block.type === 'audio'"
      class="w-[min(100%,28rem)]"
      controls
      :src="getAudioSrc(block)"
    />
    <a
      v-else-if="block.type === 'resource_link'"
      class="resource-link-chip inline-flex items-start gap-1.5 px-3 py-1.5 border border-border-subtle rounded bg-bg-surface text-brand text-sm font-medium no-underline hover:underline"
      :href="block.uri"
      target="_blank"
      rel="noreferrer noopener"
      :title="getResourceLinkTooltip(block) || undefined"
    >
      <span class="flex flex-col gap-0.5 min-w-0">
        <span class="leading-tight">{{ getResourceLinkLabel(block) }}</span>
        <span
          v-if="getResourceLinkDescription(block) || getResourceLinkSizeLabel(block)"
          class="inline-flex gap-1.5 text-text-secondary text-xs font-normal leading-tight"
        >
          <span
            v-if="getResourceLinkDescription(block)"
            class="max-w-64 overflow-hidden whitespace-nowrap text-ellipsis"
          >{{ getResourceLinkDescription(block) }}</span>
          <span v-if="getResourceLinkSizeLabel(block)" class="whitespace-nowrap">{{ getResourceLinkSizeLabel(block) }}</span>
        </span>
      </span>
      <span class="max-w-72 overflow-hidden whitespace-nowrap text-ellipsis text-text-secondary font-normal">{{ block.uri }}</span>
    </a>
    <div v-else-if="block.type === 'resource'" class="flex flex-col gap-2">
      <div class="text-text-secondary text-sm break-all">{{ getResourceUri(block) }}</div>
      <pre class="m-0 p-3 rounded overflow-x-auto bg-bg-surface text-text-secondary font-mono"><code>{{ getResourceBody(block) }}</code></pre>
    </div>
  </div>
</template>

<!-- audit-ok: max-style-rules — :deep() rules style sanitized markdown HTML rendered via v-html (h1–h4, p, ul/ol, pre, code, table, th/td, tr); these target child markup that Tailwind utilities cannot reach across the scoped boundary. -->
<style scoped>
.message-content :deep(h1),
.message-content :deep(h2),
.message-content :deep(h3),
.message-content :deep(h4) {
  margin: 0.75rem 0 0.5rem;
  font-weight: var(--weight-title);
  line-height: 1.3;
}

.message-content :deep(h1) {
  font-size: var(--text-xl);
}

.message-content :deep(h2) {
  font-size: var(--text-lg);
}

.message-content :deep(h3) {
  font-size: var(--text-md);
}

.message-content :deep(h4) {
  font-size: var(--text-base);
}

.message-content :deep(p) {
  margin: 0.5rem 0;
  font-size: var(--text-sm);
}

.message-content :deep(ol),
.message-content :deep(ul) {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}

.message-content :deep(li) {
  margin: 0.25rem 0;
  font-size: var(--text-sm);
}

.message-content :deep(pre) {
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
  background: var(--color-bg-surface);
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}

.message-content :deep(code) {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.message-content :deep(pre code) {
  font-size: var(--text-sm);
}

.message-content :deep(table) {
  display: block;
  width: 100%;
  margin: 0.75rem 0;
  overflow-x: auto;
  border-collapse: collapse;
  font-size: var(--text-sm);
}

.message-content :deep(th),
.message-content :deep(td) {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border-subtle);
  text-align: left;
}

.message-content :deep(th) {
  background: var(--color-bg-surface);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
}

.message-content :deep(td) {
  font-size: var(--text-sm);
}

.message-content :deep(tr:nth-child(even)) {
  background: var(--color-bg-app);
}

.message-content :deep(tr:hover) {
  background: var(--color-bg-surface);
}
</style>
