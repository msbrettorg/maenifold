<script setup lang="ts">
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import type { ContentBlock } from '../lib/types';
import { Card, CardContent } from './ui/card';

defineProps<{ blocks: ContentBlock[] }>();

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  const html = DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
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

function sanitizeUri(uri: string | undefined): string | undefined {
  if (!uri) return undefined;
  try {
    const url = new URL(uri);
    if (['http:', 'https:', 'memory:'].includes(url.protocol)) return uri;
    return undefined;
  } catch {
    return undefined;
  }
}

function onResourceLinkClick(event: MouseEvent, uri: string | undefined): void {
  if (!sanitizeUri(uri)) {
    event.preventDefault();
  }
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
  <Card
    v-for="(block, index) in blocks"
    :key="`${block.type}-${index}`"
    class="border-border-subtle bg-bg-surface [&+&]:mt-3"
    role="region"
    :aria-label="getBlockAriaLabel(block)"
  >
    <CardContent class="p-3">
      <div
        v-if="block.type === 'text'"
        class="leading-relaxed text-sm text-text-primary [overflow-wrap:break-word] [word-wrap:break-word] [&_h1]:text-xl [&_h1]:font-semibold [&_h1]:mt-3 [&_h1]:mb-2 [&_h2]:text-lg [&_h2]:font-semibold [&_h2]:mt-3 [&_h2]:mb-2 [&_h3]:text-base [&_h3]:font-semibold [&_h3]:mt-3 [&_h3]:mb-2 [&_h4]:text-sm [&_h4]:font-semibold [&_h4]:mt-3 [&_h4]:mb-2 [&_p]:my-2 [&_p]:text-sm [&_ul]:my-2 [&_ul]:pl-6 [&_ul]:list-disc [&_ol]:my-2 [&_ol]:pl-6 [&_ol]:list-decimal [&_li]:my-1 [&_li]:text-sm [&_pre]:p-3 [&_pre]:rounded [&_pre]:overflow-x-auto [&_pre]:bg-bg-app [&_pre]:text-text-secondary [&_pre]:font-mono [&_pre]:text-sm [&_code]:font-mono [&_code]:text-sm [&_table]:block [&_table]:w-full [&_table]:my-3 [&_table]:overflow-x-auto [&_table]:border-collapse [&_table]:text-sm [&_th]:px-3 [&_th]:py-2 [&_th]:border [&_th]:border-border-subtle [&_th]:text-left [&_th]:bg-bg-app [&_th]:font-semibold [&_td]:px-3 [&_td]:py-2 [&_td]:border [&_td]:border-border-subtle [&_td]:text-left [&_tr:nth-child(even)]:bg-bg-app/40 [&_tr:hover]:bg-bg-app [&_a]:text-brand [&_a]:underline"
        v-html="renderMarkdown(block.text)"
      />
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
        class="inline-flex items-start gap-1.5 px-3 py-1.5 border border-border-subtle rounded bg-bg-app text-brand text-sm font-medium no-underline hover:underline"
        :href="sanitizeUri(block.uri)"
        target="_blank"
        rel="noreferrer noopener"
        :title="getResourceLinkTooltip(block) || undefined"
        @click="onResourceLinkClick($event, block.uri)"
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
        <pre class="m-0 p-3 rounded overflow-x-auto bg-bg-app text-text-secondary font-mono text-sm"><code>{{ getResourceBody(block) }}</code></pre>
      </div>
    </CardContent>
  </Card>
</template>
