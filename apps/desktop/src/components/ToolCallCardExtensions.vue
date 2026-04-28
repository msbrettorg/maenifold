<script setup lang="ts">
import { computed, ref } from 'vue';
import VueJsonPretty from 'vue-json-pretty';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import MarkdownView from './MarkdownView.vue';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import type { ToolCallContentBlock, ToolCallInfo } from '../lib/types';

type JsonValue = string | number | boolean | unknown[] | Record<string, unknown> | null;
type DiffLine = { kind: 'header' | 'context' | 'add' | 'remove'; text: string };

const props = defineProps<{
  toolCall: ToolCallInfo;
  expanded: boolean;
}>();

const emit = defineEmits<{ toggle: [] }>();
const rawOpen = ref(false);
const appShellStore = useAppShellStore();
const knowledgeStore = useKnowledgeStore();

const hasContent = computed(() => Boolean(props.toolCall.content?.length));
const hasRawPayload = computed(() => Boolean(props.toolCall.rawInput || props.toolCall.rawOutput));
const hasDetails = computed(() => hasContent.value || hasRawPayload.value);
const rawPayload = computed<JsonValue>(() => ({
  tool_call: props.toolCall.rawInput ?? null,
  tool_call_update: props.toolCall.rawOutput ?? null,
}));

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isContentBlock(block: ToolCallContentBlock): block is Extract<ToolCallContentBlock, { type: 'content' }> {
  return block.type === 'content';
}

function isDiffBlock(block: ToolCallContentBlock): block is Extract<ToolCallContentBlock, { type: 'diff' }> {
  return block.type === 'diff';
}

function contentText(block: Extract<ToolCallContentBlock, { type: 'content' }>): string | null {
  const content = block.content;
  return content?.type === 'text' ? content.text : null;
}

function splitLines(text: string | null | undefined): string[] {
  return (text ?? '').split('\n');
}

function buildDiffOperations(oldLines: string[], newLines: string[]): DiffLine[] {
  const lengths = Array.from(
    { length: oldLines.length + 1 },
    () => Array<number>(newLines.length + 1).fill(0),
  );
  for (let oldIndex = oldLines.length - 1; oldIndex >= 0; oldIndex -= 1) {
    for (let newIndex = newLines.length - 1; newIndex >= 0; newIndex -= 1) {
      lengths[oldIndex][newIndex] = oldLines[oldIndex] === newLines[newIndex]
        ? lengths[oldIndex + 1][newIndex + 1] + 1
        : Math.max(lengths[oldIndex + 1][newIndex], lengths[oldIndex][newIndex + 1]);
    }
  }

  const diffLines: DiffLine[] = [];
  let oldIndex = 0;
  let newIndex = 0;
  while (oldIndex < oldLines.length && newIndex < newLines.length) {
    if (oldLines[oldIndex] === newLines[newIndex]) {
      diffLines.push({ kind: 'context', text: ` ${oldLines[oldIndex]}` });
      oldIndex += 1;
    } else if (lengths[oldIndex + 1][newIndex] >= lengths[oldIndex][newIndex + 1]) {
      diffLines.push({ kind: 'remove', text: `-${oldLines[oldIndex]}` });
      oldIndex += 1;
      continue;
    } else {
      diffLines.push({ kind: 'add', text: `+${newLines[newIndex]}` });
    }
    newIndex += 1;
  }

  while (oldIndex < oldLines.length) diffLines.push({ kind: 'remove', text: `-${oldLines[oldIndex++]}` });
  while (newIndex < newLines.length) diffLines.push({ kind: 'add', text: `+${newLines[newIndex++]}` });
  return diffLines;
}

function getUnifiedDiffLines(block: Extract<ToolCallContentBlock, { type: 'diff' }>): DiffLine[] {
  return [
    { kind: 'header', text: `--- ${block.oldText == null ? '/dev/null' : block.path}` },
    { kind: 'header', text: `+++ ${block.path}` },
    ...buildDiffOperations(splitLines(block.oldText), splitLines(block.newText)),
  ];
}

function getDiffLineClass(kind: DiffLine['kind']): string {
  switch (kind) {
    case 'header': return 'text-brand';
    case 'add': return 'text-success';
    case 'remove': return 'text-danger';
    default: return 'text-text-secondary';
  }
}

function locationLabel(location: { path: string; line?: number }): string {
  return location.line == null ? location.path : `${location.path}:${location.line}`;
}

function openLocation(path: string): void {
  if (path.startsWith('memory://')) {
    appShellStore.setActiveTab('knowledge');
    knowledgeStore.openNote(path);
    return;
  }

  if (path.startsWith('/')) {
    void window.electronAPI?.openPath?.(path);
  }
}

function blockJson(block: ToolCallContentBlock): JsonValue {
  if (
    block === null ||
    typeof block === 'string' ||
    typeof block === 'number' ||
    typeof block === 'boolean' ||
    Array.isArray(block) ||
    isRecord(block)
  ) {
    return block as JsonValue;
  }
  return String(block);
}
</script>

<template>
  <div class="mt-3 flex flex-col gap-2">
    <div v-if="toolCall.locations?.length" class="flex flex-wrap gap-1" aria-label="Tool call locations">
      <Button
        v-for="(loc, index) in toolCall.locations"
        :key="`${toolCall.toolCallId}-chip-${index}`"
        type="button"
        variant="outline"
        size="sm"
        class="h-auto max-w-full justify-start truncate px-2 py-1 font-mono text-xs"
        @click="openLocation(loc.path)"
      >
        {{ locationLabel(loc) }}
      </Button>
    </div>

    <Button
      v-if="hasDetails"
      type="button"
      variant="ghost"
      size="sm"
      class="w-fit px-2 py-1 text-xs"
      @click="emit('toggle')"
    >
      {{ expanded ? 'Hide details' : 'View details' }}
    </Button>

    <div v-if="expanded" class="flex flex-col gap-2">
      <div
        v-for="(block, blockIndex) in toolCall.content"
        :key="`${toolCall.toolCallId}-content-${blockIndex}`"
        class="rounded border border-border-subtle bg-surface-container p-2"
      >
        <MarkdownView
          v-if="isContentBlock(block) && contentText(block)"
          :content="contentText(block) ?? ''"
        />
        <div v-else-if="isDiffBlock(block)" class="flex flex-col gap-1.5">
          <Badge variant="outline" class="self-start font-mono text-xs">{{ block.path }}</Badge>
          <pre class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono text-sm"><code class="font-mono"><span
            v-for="(line, lineIndex) in getUnifiedDiffLines(block)"
            :key="`${toolCall.toolCallId}-diff-${blockIndex}-${lineIndex}`"
            :class="['block whitespace-pre', getDiffLineClass(line.kind)]"
          >{{ line.text }}
</span></code></pre>
        </div>
        <VueJsonPretty
          v-else
          :data="blockJson(block)"
          :deep="3"
          :show-line="false"
          :show-double-quotes="true"
          :show-length="true"
        />
      </div>

      <Button
        v-if="hasRawPayload"
        type="button"
        variant="outline"
        size="sm"
        class="w-fit px-2 py-1 text-xs"
        @click="rawOpen = !rawOpen"
      >
        {{ rawOpen ? 'Hide raw' : 'View raw' }}
      </Button>
      <VueJsonPretty
        v-if="rawOpen"
        :data="rawPayload"
        :deep="4"
        :show-line="false"
        :show-double-quotes="true"
        :show-length="true"
      />
    </div>
  </div>
</template>
