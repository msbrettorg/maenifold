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
  <div :id="id" class="tool-call-details" role="region" :aria-expanded="expanded">
    <div v-if="toolCall.content?.length" class="tool-call-content-list">
      <div v-for="(block, blockIndex) in toolCall.content" :key="`${toolCall.toolCallId}-content-${blockIndex}`" class="tool-content-block">
        <ChatContentBlocks v-if="isContentToolCallContentBlock(block)" :blocks="[block.content]" />
        <div v-else-if="isDiffToolCallContentBlock(block)" class="tool-content-diff">
          <div class="tool-content-label">{{ block.path }}</div>
          <pre class="tool-diff-pre"><code><span
            v-for="(line, lineIndex) in getUnifiedDiffLines(block)"
            :key="`${toolCall.toolCallId}-diff-${blockIndex}-${lineIndex}`"
            :class="['tool-diff-line', getDiffLineClass(line)]"
          >{{ line.text }}
</span></code></pre>
        </div>
        <div v-else-if="isTerminalToolCallContentBlock(block)" class="tool-content-terminal">
          <div class="tool-content-label">
            Terminal {{ block.terminalId }}
            <span v-if="getTerminalStatusLabel(block)" class="tool-terminal-status">· {{ getTerminalStatusLabel(block) }}</span>
          </div>
          <pre v-if="typeof block.output === 'string' && block.output.length > 0" class="tool-terminal-output"><code>{{ block.output }}</code></pre>
          <p v-else class="tool-terminal-placeholder">Live terminal output will appear here once terminal rendering lands.</p>
          <p v-if="block.truncated === true" class="tool-terminal-meta">Output truncated by the terminal byte limit.</p>
        </div>
        <pre v-else class="tool-content-unknown"><code>{{ JSON.stringify(block, null, 2) }}</code></pre>
      </div>
    </div>

    <Accordion
      v-if="toolCall.rawInput || toolCall.rawOutput"
      v-model="rawPayloadOpenSections"
      type="multiple"
      class="tool-call-payload-group"
    >
      <AccordionItem v-if="toolCall.rawInput" value="input" class="tool-call-payload">
        <AccordionTrigger>Raw input</AccordionTrigger>
        <AccordionContent>
          <pre><code>{{ JSON.stringify(toolCall.rawInput, null, 2) }}</code></pre>
        </AccordionContent>
      </AccordionItem>

      <AccordionItem v-if="toolCall.rawOutput" value="output" class="tool-call-payload">
        <AccordionTrigger>Raw output</AccordionTrigger>
        <AccordionContent>
          <pre><code>{{ JSON.stringify(toolCall.rawOutput, null, 2) }}</code></pre>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  </div>
</template>

<style scoped>
.tool-call-details { display: flex; flex-direction: column; gap: 0.625rem; padding: 0 0.625rem 0.625rem; }
.tool-call-content-list { display: flex; flex-direction: column; gap: 0.625rem; }
.tool-content-block {
  padding: 0.625rem;
  border: 1px solid color-mix(in srgb, var(--color-text-secondary) 25%, transparent);
  border-radius: var(--radius);
  background: var(--color-bg-elevated);
}
.tool-content-label {
  margin-bottom: 0.375rem;
  color: var(--color-text-secondary);
  font-size: var(--text-xs);
  font-weight: var(--weight-title);
}
.tool-content-unknown,
.tool-diff-pre,
.tool-terminal-output,
.tool-call-payload pre {
  margin: 0;
  padding: 0.75rem;
  overflow-x: auto;
  border-radius: var(--radius);
  background: var(--color-bg-surface);
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}
.tool-content-unknown code,
.tool-diff-pre code,
.tool-terminal-output code,
.tool-call-payload code { font-family: var(--font-mono); font-size: var(--text-sm); }
.tool-diff-line { display: block; white-space: pre; }
.diff-line-header { color: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-status-active)); }
.diff-line-context { color: inherit; }
.diff-line-add { color: color-mix(in srgb, var(--color-text-primary) 68%, var(--color-status-completed)); }
.diff-line-remove { color: color-mix(in srgb, var(--color-text-primary) 68%, var(--color-danger)); }
.tool-terminal-placeholder,
.tool-terminal-meta { margin: 0; color: var(--color-text-secondary); }
.tool-terminal-status { font-weight: var(--weight-display); }
.tool-call-payload {
  border: 1px solid color-mix(in srgb, var(--color-text-secondary) 35%, transparent);
  border-radius: var(--radius);
  background: var(--color-bg-elevated);
}
.tool-call-payload summary { padding: 0.5rem 0.75rem; font-weight: var(--weight-title); cursor: pointer; }
.tool-call-payload pre { margin: 0 0.75rem 0.75rem; }
</style>
