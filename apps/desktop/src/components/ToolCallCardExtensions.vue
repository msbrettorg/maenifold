<script setup lang="ts">
import { computed, ref } from 'vue';
import VueJsonPretty from 'vue-json-pretty';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import ChatContentBlocks from './ChatContentBlocks.vue';
import { useToolCallTerminal } from '../composables/useToolCallTerminal';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import type { ToolCallContentBlock, ToolCallInfo } from '../lib/types';
import { diffLines } from 'diff';

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
const {
  isTerminalBlock,
  terminalExitCode,
  terminalId,
  terminalStatusLabel,
  terminalOutputState,
  terminalRenderedOutput,
  terminalRenderedStatusLabel,
  terminalRenderedBadgeVariant,
  terminalIsTruncated,
} = useToolCallTerminal();

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

function buildDiffOperations(oldText: string | null | undefined, newText: string | null | undefined): DiffLine[] {
  const changes = diffLines(oldText ?? '', newText ?? '');
  const result: DiffLine[] = [];
  for (const change of changes) {
    const lines = change.value.replace(/\n$/, '').split('\n');
    const kind: DiffLine['kind'] = change.added ? 'add' : change.removed ? 'remove' : 'context';
    const prefix = change.added ? '+' : change.removed ? '-' : ' ';
    for (const line of lines) {
      result.push({ kind, text: `${prefix}${line}` });
    }
  }
  return result;
}

function getUnifiedDiffLines(block: Extract<ToolCallContentBlock, { type: 'diff' }>): DiffLine[] {
  return [
    { kind: 'header', text: `--- ${block.oldText == null ? '/dev/null' : block.path}` },
    { kind: 'header', text: `+++ ${block.path}` },
    ...buildDiffOperations(block.oldText, block.newText),
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

function terminalBadgeVariant(block: Extract<ToolCallContentBlock, { type: 'terminal' }>): 'outline' | 'destructive' {
  const exitCode = terminalExitCode(block);
  return exitCode != null && exitCode !== 0 ? 'destructive' : 'outline';
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

  // Absolute-path navigation deferred to T-244.72 permission gate.
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
        <ChatContentBlocks
          v-if="isContentBlock(block)"
          :blocks="[block.content]"
        />
        <div v-else-if="isDiffBlock(block)" class="flex flex-col gap-1.5" data-testid="tool-call-diff">
          <Badge variant="outline" class="self-start font-mono text-xs">{{ block.path }}</Badge>
          <pre class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono text-sm"><code class="font-mono"><span
            v-for="(line, lineIndex) in getUnifiedDiffLines(block)"
            :key="`${toolCall.toolCallId}-diff-${blockIndex}-${lineIndex}`"
            :class="['block whitespace-pre', getDiffLineClass(line.kind)]"
          >{{ line.text }}
</span></code></pre>
        </div>
        <div v-else-if="isTerminalBlock(block)" class="flex flex-col gap-1.5">
          <div
            v-if="terminalId(block) && !terminalOutputState(block)"
            class="flex flex-wrap items-center gap-2 rounded bg-surface px-3 py-2 text-sm text-text-secondary"
            data-testid="terminal-reference"
          >
            <span class="font-medium text-text-primary">Terminal: {{ terminalId(block) }}</span>
            <Badge :variant="terminalBadgeVariant(block)" class="text-xs">
              {{ terminalStatusLabel(block) ?? 'status pending' }}
            </Badge>
            <span class="text-xs">Live terminal output is referenced by terminalId.</span>
          </div>
          <div class="flex items-center gap-2">
            <Badge
              v-if="terminalRenderedStatusLabel(block) && (!terminalId(block) || terminalOutputState(block))"
              :variant="terminalRenderedBadgeVariant(block)"
              class="text-xs"
              data-testid="terminal-exit-code"
            >
              {{ terminalRenderedStatusLabel(block) }}
            </Badge>
          </div>
          <pre
            v-if="terminalRenderedOutput(block).length > 0"
            class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono font-tabular-nums text-sm text-text-secondary"
            data-testid="terminal-output"
          ><code class="font-mono font-tabular-nums">{{ terminalRenderedOutput(block) }}</code></pre>
          <p v-if="terminalIsTruncated(block)" class="m-0 text-xs italic text-text-secondary" data-testid="terminal-truncated">
            Output truncated.
          </p>
        </div>
        <pre
          v-else
          class="m-0 overflow-x-auto rounded bg-surface p-3 font-mono text-sm text-text-secondary"
          data-testid="unknown-tool-call-block"
        ><code class="font-mono">{{ JSON.stringify(blockJson(block), null, 2) }}</code></pre>
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
