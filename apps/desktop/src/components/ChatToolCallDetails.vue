<script setup lang="ts">
import { computed, ref } from 'vue';
import { ChevronDown, ChevronRight } from 'lucide-vue-next';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import ChatContentBlocks from './ChatContentBlocks.vue';
import type { ToolCallContentBlock, ToolCallInfo } from '../lib/types';

type DiffLine = {
  kind: 'header' | 'context' | 'add' | 'remove';
  text: string;
};

defineProps<{
  id?: string;
  toolCall: ToolCallInfo;
  expanded: boolean;
}>();

const rawInputOpen = ref(false);
const rawOutputOpen = ref(false);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isContentToolCallContentBlock(
  block: ToolCallContentBlock
): block is Extract<ToolCallContentBlock, { type: 'content' }> & { content: NonNullable<Extract<ToolCallContentBlock, { type: 'content' }>['content']> } {
  return block.type === 'content' && block.content != null;
}

function isDiffToolCallContentBlock(
  block: ToolCallContentBlock
): block is Extract<ToolCallContentBlock, { type: 'diff' }> {
  return block.type === 'diff';
}

function isTerminalToolCallContentBlock(
  block: ToolCallContentBlock
): block is Extract<ToolCallContentBlock, { type: 'terminal' }> {
  return block.type === 'terminal';
}

function splitDiffLines(text: string | null | undefined): string[] {
  return (text ?? '').split('\n');
}

function buildDiffOperations(oldLines: string[], newLines: string[]): DiffLine[] {
  const lengths = Array.from({ length: oldLines.length + 1 }, () => Array<number>(newLines.length + 1).fill(0));
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
      newIndex += 1;
    } else if (lengths[oldIndex + 1][newIndex] >= lengths[oldIndex][newIndex + 1]) {
      diffLines.push({ kind: 'remove', text: `-${oldLines[oldIndex]}` });
      oldIndex += 1;
    } else {
      diffLines.push({ kind: 'add', text: `+${newLines[newIndex]}` });
      newIndex += 1;
    }
  }

  while (oldIndex < oldLines.length) diffLines.push({ kind: 'remove', text: `-${oldLines[oldIndex++]}` });
  while (newIndex < newLines.length) diffLines.push({ kind: 'add', text: `+${newLines[newIndex++]}` });
  return diffLines;
}

function getUnifiedDiffLines(block: Extract<ToolCallContentBlock, { type: 'diff' }>): DiffLine[] {
  return [
    { kind: 'header', text: `--- ${block.oldText == null ? '/dev/null' : block.path}` },
    { kind: 'header', text: `+++ ${block.path}` },
    ...buildDiffOperations(splitDiffLines(block.oldText), splitDiffLines(block.newText)),
  ];
}

function getDiffLineClass(kind: DiffLine['kind']): string {
  switch (kind) {
    case 'header':
      return 'text-brand';
    case 'add':
      return 'text-success';
    case 'remove':
      return 'text-danger';
    case 'context':
    default:
      return 'text-text-secondary';
  }
}

function getTerminalStatusLabel(block: Extract<ToolCallContentBlock, { type: 'terminal' }>): string | null {
  const exitStatus = isRecord(block.exitStatus) ? block.exitStatus : null;
  const exitCode = typeof exitStatus?.exitCode === 'number' ? exitStatus.exitCode : null;
  const signal = typeof exitStatus?.signal === 'string' && exitStatus.signal.length > 0 ? exitStatus.signal : null;
  if (signal) return `signal: ${signal}`;
  if (exitCode != null) return `exit ${exitCode}`;
  return block.released === true ? 'released' : null;
}

function getTerminalBadgeVariant(
  block: Extract<ToolCallContentBlock, { type: 'terminal' }>
): 'default' | 'secondary' | 'destructive' | 'outline' {
  const exitStatus = isRecord(block.exitStatus) ? block.exitStatus : null;
  const exitCode = typeof exitStatus?.exitCode === 'number' ? exitStatus.exitCode : null;
  const signal = typeof exitStatus?.signal === 'string' && exitStatus.signal.length > 0 ? exitStatus.signal : null;
  if (signal) return 'destructive';
  if (exitCode != null && exitCode !== 0) return 'destructive';
  if (exitCode === 0) return 'secondary';
  return 'outline';
}

const hasRawPayload = computed(() => true);
</script>

<template>
  <div
    :id="id"
    class="flex flex-col gap-2.5 px-2.5 pb-2.5"
    role="region"
    :aria-expanded="expanded"
  >
    <div v-if="toolCall.content?.length" class="flex flex-col gap-2.5">
      <div
        v-for="(block, blockIndex) in toolCall.content"
        :key="`${toolCall.toolCallId}-content-${blockIndex}`"
        class="rounded border border-border-subtle bg-surface-container p-2.5"
      >
        <ChatContentBlocks
          v-if="isContentToolCallContentBlock(block)"
          :blocks="[block.content]"
        />

        <div v-else-if="isDiffToolCallContentBlock(block)" class="flex flex-col gap-1.5">
          <Badge variant="outline" class="self-start font-mono text-xs">
            {{ block.path }}
          </Badge>
          <pre class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono text-sm text-text-secondary"><code class="font-mono"><span
            v-for="(line, lineIndex) in getUnifiedDiffLines(block)"
            :key="`${toolCall.toolCallId}-diff-${blockIndex}-${lineIndex}`"
            :class="['block whitespace-pre', getDiffLineClass(line.kind)]"
          >{{ line.text }}
</span></code></pre>
        </div>

        <div v-else-if="isTerminalToolCallContentBlock(block)" class="flex flex-col gap-1.5">
          <div class="flex items-center gap-2">
            <Badge variant="outline" class="font-mono text-xs">
              Terminal {{ block.terminalId }}
            </Badge>
            <Badge
              v-if="getTerminalStatusLabel(block)"
              :variant="getTerminalBadgeVariant(block)"
              class="text-xs"
            >
              {{ getTerminalStatusLabel(block) }}
            </Badge>
          </div>
          <pre
            v-if="typeof block.output === 'string' && block.output.length > 0"
            class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono text-sm text-text-secondary"
          ><code class="font-mono">{{ block.output }}</code></pre>
          <div v-else class="flex flex-col gap-2">
            <Skeleton class="h-4 w-3/4 rounded" />
            <Skeleton class="h-4 w-2/3 rounded" />
            <p class="m-0 text-xs italic text-text-secondary">
              Live terminal output will appear here once terminal rendering lands.
            </p>
          </div>
          <p v-if="block.truncated === true" class="m-0 text-xs italic text-text-secondary">
            Output truncated by the terminal byte limit.
          </p>
        </div>

        <pre
          v-else
          class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono text-sm text-text-secondary"
        ><code class="font-mono">{{ JSON.stringify(block, null, 2) }}</code></pre>
      </div>
    </div>

    <div v-if="hasRawPayload && (toolCall.rawInput || toolCall.rawOutput)" class="flex flex-col gap-2">
      <Collapsible
        v-if="toolCall.rawInput"
        v-model:open="rawInputOpen"
        class="overflow-hidden rounded border border-border-subtle bg-surface-container"
      >
        <CollapsibleTrigger
          as-child
          :aria-controls="`${toolCall.toolCallId}-raw-input`"
        >
          <Badge
            variant="ghost"
            class="flex w-full cursor-pointer items-center justify-start gap-2 rounded-none border-none px-3 py-2 text-left text-xs font-medium text-text-secondary"
          >
            <ChevronDown v-if="rawInputOpen" class="h-3 w-3" />
            <ChevronRight v-else class="h-3 w-3" />
            <span class="flex-1">Raw input</span>
          </Badge>
        </CollapsibleTrigger>
        <CollapsibleContent :id="`${toolCall.toolCallId}-raw-input`">
          <pre class="m-0 mx-3 mb-3 overflow-x-auto rounded bg-surface p-3 font-mono text-sm text-text-secondary"><code class="font-mono">{{ JSON.stringify(toolCall.rawInput, null, 2) }}</code></pre>
        </CollapsibleContent>
      </Collapsible>

      <Collapsible
        v-if="toolCall.rawOutput"
        v-model:open="rawOutputOpen"
        class="overflow-hidden rounded border border-border-subtle bg-surface-container"
      >
        <CollapsibleTrigger
          as-child
          :aria-controls="`${toolCall.toolCallId}-raw-output`"
        >
          <Badge
            variant="ghost"
            class="flex w-full cursor-pointer items-center justify-start gap-2 rounded-none border-none px-3 py-2 text-left text-xs font-medium text-text-secondary"
          >
            <ChevronDown v-if="rawOutputOpen" class="h-3 w-3" />
            <ChevronRight v-else class="h-3 w-3" />
            <span class="flex-1">Raw output</span>
          </Badge>
        </CollapsibleTrigger>
        <CollapsibleContent :id="`${toolCall.toolCallId}-raw-output`">
          <pre class="m-0 mx-3 mb-3 overflow-x-auto rounded bg-surface p-3 font-mono text-sm text-text-secondary"><code class="font-mono">{{ JSON.stringify(toolCall.rawOutput, null, 2) }}</code></pre>
        </CollapsibleContent>
      </Collapsible>
    </div>
  </div>
</template>
