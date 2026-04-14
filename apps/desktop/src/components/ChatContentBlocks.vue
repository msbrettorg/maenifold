<script setup lang="ts">
import { marked } from 'marked';
import type { ContentBlock } from '../lib/types';

defineProps<{ blocks: ContentBlock[] }>();

function renderMarkdown(content: string): string {
  return marked.parse(content, { async: false }) as string;
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
    class="message-content-block"
    role="region"
    :aria-label="getBlockAriaLabel(block)"
  >
    <div v-if="block.type === 'text'" class="message-content" v-html="renderMarkdown(block.text)" />
    <img v-else-if="block.type === 'image'" class="message-image" :src="getImageSrc(block)" :alt="block.uri ?? 'Message image'" />
    <audio v-else-if="block.type === 'audio'" class="message-audio" controls :src="getAudioSrc(block)" />
    <a
      v-else-if="block.type === 'resource_link'"
      class="resource-link-chip"
      :href="block.uri"
      target="_blank"
      rel="noreferrer noopener"
      :title="getResourceLinkTooltip(block) || undefined"
    >
      <span class="resource-link-chip__content">
        <span class="resource-link-chip__label">{{ getResourceLinkLabel(block) }}</span>
        <span v-if="getResourceLinkDescription(block) || getResourceLinkSizeLabel(block)" class="resource-link-chip__meta">
          <span v-if="getResourceLinkDescription(block)" class="resource-link-chip__description">{{ getResourceLinkDescription(block) }}</span>
          <span v-if="getResourceLinkSizeLabel(block)" class="resource-link-chip__size">{{ getResourceLinkSizeLabel(block) }}</span>
        </span>
      </span>
      <span class="resource-link-uri">{{ block.uri }}</span>
    </a>
    <div v-else-if="block.type === 'resource'" class="resource-block">
      <div class="resource-label">{{ getResourceUri(block) }}</div>
      <pre class="resource-content"><code>{{ getResourceBody(block) }}</code></pre>
    </div>
  </div>
</template>

<style scoped>
.message-content-block + .message-content-block { margin-top: 0.75rem; }
.message-content {
  line-height: 1.5;
  overflow-wrap: break-word;
  word-wrap: break-word;
}
.message-content :deep(h1),
.message-content :deep(h2),
.message-content :deep(h3),
.message-content :deep(h4) {
  margin: 0.75rem 0 0.5rem;
  font-weight: var(--weight-title);
  line-height: 1.3;
}
.message-content :deep(h1) { font-size: var(--text-xl); }
.message-content :deep(h2) { font-size: var(--text-lg); }
.message-content :deep(h3) { font-size: var(--text-md); }
.message-content :deep(h4) { font-size: var(--text-base); }
.message-content :deep(p) {
  margin: 0.5rem 0;
  font-size: var(--text-sm);
}
.message-content :deep(ol),
.message-content :deep(ul) { margin: 0.5rem 0; padding-left: 1.5rem; }
.message-content :deep(li) {
  margin: 0.25rem 0;
  font-size: var(--text-sm);
}
.message-content :deep(pre) {
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
  background: var(--muted);
  color: var(--muted-foreground);
}
.message-content :deep(code) {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}
.message-content :deep(pre code) { font-size: var(--text-sm); }
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
  border: 1px solid var(--border);
  text-align: left;
}
.message-content :deep(th) {
  background: var(--muted);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
}
.message-content :deep(td) { font-size: var(--text-sm); }
.message-content :deep(tr:nth-child(even)) { background: var(--background); }
.message-content :deep(tr:hover) { background: var(--muted); }
.message-image {
  display: block;
  max-width: min(100%, 32rem);
  max-height: 24rem;
  border: 1px solid var(--border);
  border-radius: 6px;
}
.message-audio { width: min(100%, 28rem); }
.resource-link-chip {
  display: inline-flex;
  align-items: flex-start;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--muted);
  color: var(--ring);
  text-decoration: none;
  font-size: var(--text-sm);
  font-weight: var(--weight-display);
}
.resource-link-chip:hover { text-decoration: underline; }
.resource-link-chip__content { display: flex; flex-direction: column; gap: 0.125rem; min-width: 0; }
.resource-link-chip__label { line-height: 1.1; }
.resource-link-chip__meta {
  display: inline-flex;
  gap: 0.375rem;
  color: var(--muted-foreground);
  font-size: var(--text-xs);
  font-weight: var(--weight-body);
  line-height: 1.1;
}
.resource-link-chip__description {
  max-width: 16rem;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.resource-link-chip__size,
.resource-link-uri { white-space: nowrap; }
.resource-link-uri {
  max-width: 18rem;
  overflow: hidden;
  color: var(--muted-foreground);
  text-overflow: ellipsis;
  font-weight: var(--weight-body);
}
.resource-block { display: flex; flex-direction: column; gap: 0.5rem; }
.resource-label { color: var(--muted-foreground); font-size: var(--text-sm); word-break: break-all; }
.resource-content {
  margin: 0;
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
  background: var(--muted);
  color: var(--muted-foreground);
}
</style>
