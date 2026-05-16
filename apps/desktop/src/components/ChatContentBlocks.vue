<script setup lang="ts">
import type { ContentBlock } from '../lib/types';
import { dispatchResource } from '../lib/resource-mime-dispatcher';
import type { ResourceContent } from '../lib/resource-mime-dispatcher';
import MarkdownView from './MarkdownView.vue';
import ResourceLinkPreview from './ResourceLinkPreview.vue';
import ResourcePreview from './ResourcePreview.vue';
import { Card, CardContent } from './ui/card';

defineProps<{ blocks: ContentBlock[] }>();

function getImageSrc(block: ContentBlock): string {
  return block.type === 'image' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getAudioSrc(block: ContentBlock): string {
  return block.type === 'audio' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getResourceUri(block: ContentBlock): string {
  return block.type === 'resource' ? block.resource.uri : '';
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
      <MarkdownView
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
      <ResourceLinkPreview
        v-else-if="block.type === 'resource_link'"
        :block="block"
      />
      <div v-else-if="block.type === 'resource'" class="flex flex-col gap-2">
        <div class="text-text-secondary text-sm break-all">{{ getResourceUri(block) }}</div>
        <ResourcePreview :dispatched="dispatchResource(block.resource as ResourceContent)" />
      </div>
    </CardContent>
  </Card>
</template>
