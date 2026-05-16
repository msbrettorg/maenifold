<script setup lang="ts">
import { computed, ref } from 'vue';
import { Download } from 'lucide-vue-next';
import type { DispatchedResource } from '../lib/resource-mime-dispatcher';
import MarkdownView from './MarkdownView.vue';
import VideoBlock from './VideoBlock.vue';
import { Button } from './ui/button';
import { Card, CardContent } from './ui/card';

const props = defineProps<{
  dispatched: DispatchedResource;
}>();

const imageExpanded = ref(false);

const codeMarkdown = computed(() => {
  if (props.dispatched.kind !== 'code') return '';
  const fence = props.dispatched.text.includes('~~~') ? '````' : '~~~';
  return `${fence}${props.dispatched.language}\n${props.dispatched.text}\n${fence}`;
});

const jsonSize = computed(() => {
  if (props.dispatched.kind !== 'json') return '';
  return formatFileSize(props.dispatched.pretty.length);
});

function formatFileSize(bytes: number): string {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
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
  <MarkdownView
    v-if="dispatched.kind === 'markdown'"
    :content="dispatched.text"
  />
  <details
    v-else-if="dispatched.kind === 'json'"
    class="rounded border border-border-subtle bg-bg-app p-3"
    open
  >
    <summary class="cursor-pointer text-sm font-medium text-text-primary">
      JSON ({{ jsonSize }})
    </summary>
    <pre class="mt-2 m-0 overflow-x-auto text-sm text-text-secondary"><code>{{ dispatched.pretty }}</code></pre>
  </details>
  <MarkdownView
    v-else-if="dispatched.kind === 'code'"
    :content="codeMarkdown"
  />
  <div
    v-else-if="dispatched.kind === 'image'"
    class="flex flex-col items-start gap-2"
  >
    <Button
      variant="ghost"
      class="h-auto max-w-full p-0"
      :aria-expanded="imageExpanded"
      @click="imageExpanded = !imageExpanded"
    >
      <img
        class="block max-w-[min(100%,32rem)] rounded border border-border-subtle"
        :class="imageExpanded ? 'max-h-[80vh]' : 'max-h-96'"
        :src="dispatched.src"
        :alt="dispatched.alt"
      />
    </Button>
    <span class="text-xs text-text-secondary">
      {{ imageExpanded ? 'Click image to collapse' : 'Click image to enlarge' }}
    </span>
  </div>
  <VideoBlock
    v-else-if="dispatched.kind === 'video'"
    :src="dispatched.src"
    :mime-type="dispatched.mimeType"
  />
  <audio
    v-else-if="dispatched.kind === 'audio'"
    class="w-[min(100%,28rem)]"
    controls
    :src="dispatched.src"
  />
  <Card
    v-else-if="dispatched.kind === 'too-large'"
    size="sm"
    class="max-w-full rounded-2xl border-border-subtle bg-bg-app shadow-none"
  >
    <CardContent class="flex min-w-0 items-start gap-3 p-3">
      <Download class="mt-0.5 size-4 shrink-0 text-text-secondary" aria-hidden="true" />
      <span class="flex min-w-0 flex-col gap-0.5">
        <span class="truncate text-sm font-medium text-text-primary">{{ dispatched.name }}</span>
        <span class="text-xs text-text-secondary">
          {{ formatFileSize(dispatched.size) }} • {{ dispatched.mimeType }}
        </span>
        <span class="text-xs text-text-tertiary">
          Preview too large — open in external viewer
        </span>
      </span>
    </CardContent>
  </Card>
  <Card
    v-else
    size="sm"
    class="max-w-full rounded-2xl border-border-subtle bg-bg-app shadow-none"
  >
    <CardContent class="flex min-w-0 items-start gap-3 p-3">
      <Download class="mt-0.5 size-4 shrink-0 text-text-secondary" aria-hidden="true" />
      <span class="flex min-w-0 flex-col gap-0.5">
        <span class="truncate text-sm font-medium text-text-primary">{{ dispatched.name }}</span>
        <span class="text-xs text-text-secondary">
          <span v-if="dispatched.size != null">{{ formatFileSize(dispatched.size) }} • </span>{{ dispatched.uri }}
        </span>
      </span>
    </CardContent>
  </Card>
</template>
