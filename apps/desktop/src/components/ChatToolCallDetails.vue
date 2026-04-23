<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
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

const rawInputExpanded = ref(false);
const rawOutputExpanded = ref(false);

const rawPayloadOpenSections = computed<string[]>({
  get() {
    const open: string[] = [];
    if (rawInputExpanded.value) open.push('input');
    if (rawOutputExpanded.value) open.push('output');
    return open;
  },
  set(values: string[]) {
    rawInputExpanded.value = values.includes('input');
    rawOutputExpanded.value = values.includes('output');
  },
});

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

function getDiffLineClass(line: DiffLine): string {
  return `diff-line-${line.kind}`;
}

function getTerminalStatusLabel(block: Extract<ToolCallContentBlock, { type: 'terminal' }>): string | null {
  const exitStatus = isRecord(block.exitStatus) ? block.exitStatus : null;
  const exitCode = typeof exitStatus?.exitCode === 'number' ? exitStatus.exitCode : null;
  const signal = typeof exitStatus?.signal === 'string' && exitStatus.signal.length > 0 ? exitStatus.signal : null;
  if (signal) return `signal: ${signal}`;
  if (exitCode != null) return `exit ${exitCode}`;
  return block.released === true ? 'released' : null;
}
</script>

<template>
  <div :id="id" class="flex flex-col gap-[0.625rem] px-[0.625rem] pb-[0.625rem]" role="region" :aria-expanded="expanded">
    <div v-if="toolCall.content?.length" class="flex flex-col gap-[0.625rem]">
      <div v-for="(block, blockIndex) in toolCall.content" :key="`${toolCall.toolCallId}-content-${blockIndex}`" class="tool-content-block p-[0.625rem] rounded-[var(--radius)] bg-bg-elevated">
        <ChatContentBlocks v-if="isContentToolCallContentBlock(block)" :blocks="[block.content]" />
        <div v-else-if="isDiffToolCallContentBlock(block)">
          <div class="mb-1.5 text-text-secondary text-xs font-[number:var(--weight-title)]">{{ block.path }}</div>
          <pre class="m-0 p-3 overflow-x-auto rounded-[var(--radius)] bg-bg-surface text-text-secondary font-[var(--font-mono)]"><code class="font-[var(--font-mono)] text-sm"><span
            v-for="(line, lineIndex) in getUnifiedDiffLines(block)"
            :key="`${toolCall.toolCallId}-diff-${blockIndex}-${lineIndex}`"
            :class="['block whitespace-pre', getDiffLineClass(line)]"
          >{{ line.text }}
</span></code></pre>
        </div>
        <div v-else-if="isTerminalToolCallContentBlock(block)">
          <div class="mb-1.5 text-text-secondary text-xs font-[number:var(--weight-title)]">
            Terminal {{ block.terminalId }}
            <span v-if="getTerminalStatusLabel(block)" class="font-[number:var(--weight-display)]">· {{ getTerminalStatusLabel(block) }}</span>
          </div>
          <pre v-if="typeof block.output === 'string' && block.output.length > 0" class="m-0 p-3 overflow-x-auto rounded-[var(--radius)] bg-bg-surface text-text-secondary font-[var(--font-mono)]"><code class="font-[var(--font-mono)] text-sm">{{ block.output }}</code></pre>
          <p v-else class="m-0 text-text-secondary">Live terminal output will appear here once terminal rendering lands.</p>
          <p v-if="block.truncated === true" class="m-0 text-text-secondary">Output truncated by the terminal byte limit.</p>
        </div>
        <pre v-else class="m-0 p-3 overflow-x-auto rounded-[var(--radius)] bg-bg-surface text-text-secondary font-[var(--font-mono)]"><code class="font-[var(--font-mono)] text-sm">{{ JSON.stringify(block, null, 2) }}</code></pre>
      </div>
    </div>

    <Accordion
      v-if="toolCall.rawInput || toolCall.rawOutput"
      v-model="rawPayloadOpenSections"
      type="multiple"
    >
      <AccordionItem v-if="toolCall.rawInput" value="input" class="tool-call-payload rounded-[var(--radius)] bg-bg-elevated">
        <AccordionTrigger>Raw input</AccordionTrigger>
        <AccordionContent>
          <pre class="m-0 mx-3 mb-3 p-3 overflow-x-auto rounded-[var(--radius)] bg-bg-surface text-text-secondary font-[var(--font-mono)]"><code class="font-[var(--font-mono)] text-sm">{{ JSON.stringify(toolCall.rawInput, null, 2) }}</code></pre>
        </AccordionContent>
      </AccordionItem>

      <AccordionItem v-if="toolCall.rawOutput" value="output" class="tool-call-payload rounded-[var(--radius)] bg-bg-elevated">
        <AccordionTrigger>Raw output</AccordionTrigger>
        <AccordionContent>
          <pre class="m-0 mx-3 mb-3 p-3 overflow-x-auto rounded-[var(--radius)] bg-bg-surface text-text-secondary font-[var(--font-mono)]"><code class="font-[var(--font-mono)] text-sm">{{ JSON.stringify(toolCall.rawOutput, null, 2) }}</code></pre>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  </div>
</template>

<!-- audit-ok: max-style-rules — color-mix() borders and diff-line accent colors are static design tokens (theme-aware blends of text + status colors) that Tailwind utilities cannot express; rules limited to color-mix() declarations. -->
<style scoped>
.tool-content-block {
  border: 1px solid color-mix(in srgb, var(--color-text-secondary) 25%, transparent);
}
.tool-call-payload {
  border: 1px solid color-mix(in srgb, var(--color-text-secondary) 35%, transparent);
}
.diff-line-header { color: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-status-active)); }
.diff-line-context { color: inherit; }
.diff-line-add { color: color-mix(in srgb, var(--color-text-primary) 68%, var(--color-status-completed)); }
.diff-line-remove { color: color-mix(in srgb, var(--color-text-primary) 68%, var(--color-danger)); }
</style>
