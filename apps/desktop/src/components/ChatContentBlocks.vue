<script setup lang="ts">
import type { ContentBlock } from '../lib/types';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';
import { Card, CardContent } from './ui/card';

defineProps<{ blocks: ContentBlock[] }>();

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
    class="min-w-0 max-w-full border-border-subtle bg-bg-surface [&+&]:mt-3"
    role="region"
    :aria-label="getBlockAriaLabel(block)"
  >
    <CardContent class="min-w-0 p-3">
      <KnowledgeNoteRenderer
        v-if="block.type === 'text'"
        :content="block.text"
        class="text-text-primary"
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
