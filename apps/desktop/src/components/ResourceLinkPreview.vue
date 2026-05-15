<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Code2,
  File,
  FileAudio,
  FileImage,
  FileJson,
  FileText,
  Loader2,
} from 'lucide-vue-next';
import type { Component } from 'vue';
import type { ResourceLinkContentBlock } from '../lib/types';
import { dispatchResource } from '../lib/resource-mime-dispatcher';
import type { DispatchedResource, ResourceContent } from '../lib/resource-mime-dispatcher';
import { sanitizeResourceErrorMessage, warnRawResourceErrorInDev } from '../lib/error-sanitizer';
import { useMcpResources } from '../composables/useMcpResources';
import { getErrorMessage } from '../composables/mcpTransport';
import { Alert, AlertDescription } from './ui/alert';
import { Button } from './ui/button';
import ResourcePreview from './ResourcePreview.vue';

const props = defineProps<{
  block: ResourceLinkContentBlock;
}>();

const isExpanded = ref(false);
const isLoading = ref(false);
const errorMessage = ref<string | null>(null);
const dispatched = ref<DispatchedResource | null>(null);

const label = computed(() => props.block.title || props.block.name || props.block.uri);
const description = computed(() => {
  const trimmed = props.block.description?.trim();
  return trimmed ? trimmed : null;
});
const sizeLabel = computed(() => (props.block.size != null ? formatFileSize(props.block.size) : null));
const icon = computed<Component>(() => iconForMime(props.block.mimeType ?? undefined));

async function togglePreview(): Promise<void> {
  if (isExpanded.value) {
    isExpanded.value = false;
    return;
  }

  if (dispatched.value || errorMessage.value) {
    isExpanded.value = true;
    return;
  }

  isLoading.value = true;
  errorMessage.value = null;
  try {
    const content = await useMcpResources().readResource(props.block.uri);
    dispatched.value = dispatchResource(withDescriptorFallbacks(content));
    isExpanded.value = true;
  } catch (error) {
    const rawError = getErrorMessage(error);
    warnRawResourceErrorInDev(rawError);
    errorMessage.value = sanitizeResourceErrorMessage(rawError);
    isExpanded.value = true;
  } finally {
    isLoading.value = false;
  }
}

function withDescriptorFallbacks(content: ResourceContent): ResourceContent {
  return {
    ...content,
    uri: content.uri || props.block.uri,
    mimeType: content.mimeType || props.block.mimeType || undefined,
    name: content.name || props.block.title || props.block.name || undefined,
    size: content.size ?? numberSize(props.block.size),
  };
}

function numberSize(size: number | bigint | null | undefined): number | undefined {
  if (typeof size === 'number') return size;
  if (typeof size === 'bigint' && size <= BigInt(Number.MAX_SAFE_INTEGER)) return Number(size);
  return undefined;
}

function iconForMime(mimeType: string | undefined): Component {
  const normalized = mimeType?.toLowerCase() ?? '';
  if (normalized === 'text/markdown') return FileText;
  if (normalized === 'application/json') return FileJson;
  if (normalized.startsWith('image/')) return FileImage;
  if (normalized.startsWith('audio/')) return FileAudio;
  if (normalized.startsWith('text/') || normalized.includes('typescript') || normalized.includes('javascript')) {
    return Code2;
  }
  return File;
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
  const rounded = Number.isInteger(value) ? String(value) : value < 10 ? value.toFixed(1) : String(Math.round(value));
  return `${rounded} ${units[unitIndex]}`;
}
</script>

<template>
  <div class="flex min-w-0 flex-col gap-2">
    <Button
      data-testid="resource-link-chip"
      variant="outline"
      class="h-auto max-w-full justify-start px-3 py-2 text-left"
      :loading="isLoading"
      :aria-expanded="isExpanded"
      @click="togglePreview"
    >
      <Loader2 v-if="isLoading" class="size-4 animate-spin" aria-hidden="true" />
      <component :is="icon" v-else class="size-4" aria-hidden="true" />
      <span class="flex min-w-0 flex-col gap-0.5">
        <span class="truncate leading-tight">{{ label }}</span>
        <span class="flex min-w-0 gap-1.5 text-xs font-normal text-text-secondary">
          <span v-if="description" class="max-w-64 truncate">{{ description }}</span>
          <span v-if="sizeLabel" class="whitespace-nowrap">{{ sizeLabel }}</span>
          <span class="max-w-72 truncate">{{ block.uri }}</span>
        </span>
      </span>
    </Button>
    <ResourcePreview
      v-if="isExpanded && dispatched"
      :dispatched="dispatched"
    />
    <Alert
      v-else-if="isExpanded && errorMessage"
      variant="destructive"
    >
      <AlertDescription>{{ errorMessage }}</AlertDescription>
    </Alert>
  </div>
</template>
