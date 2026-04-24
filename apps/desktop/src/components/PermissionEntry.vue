<script setup lang="ts">
import { computed } from 'vue';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { PermissionRequest, ToolCallContentBlock } from '../lib/types';

const props = defineProps<{ request: PermissionRequest }>();
const emit = defineEmits<{ select: [optionId: string]; cancel: [] }>();

type ContextRow = { label: string; value: string };
type ContentBlockView = { label: string; body: string };

const isPrimitive = (value: unknown): value is string | number | boolean | bigint =>
  typeof value === 'string' ||
  typeof value === 'number' ||
  typeof value === 'boolean' ||
  typeof value === 'bigint';

function formatValue(value: unknown): string | null {
  if (isPrimitive(value)) return String(value);
  if (Array.isArray(value) && value.every(isPrimitive)) return value.map(String).join(', ');
  return null;
}

function pickValue(payload: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const value = formatValue(payload[key]);
    if (value) return value;
  }
  return null;
}

function pickList(payload: Record<string, unknown>, keys: string[]): string[] {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value)) continue;
    const items = value
      .map((entry) => (isPrimitive(entry) ? String(entry) : null))
      .filter((entry): entry is string => entry !== null);
    if (items.length) return items;
  }
  return [];
}

function truncate(value: string, max = 240): string {
  const normalized = value.trim();
  return normalized.length <= max ? normalized : `${normalized.slice(0, max).trimEnd()}…`;
}

function formatCommand(payload: Record<string, unknown>): string | null {
  const command = pickValue(payload, ['command', 'cmd', 'program']);
  const args = pickList(payload, ['args', 'argv']);
  return !command && !args.length ? null : [command, ...args].filter(Boolean).join(' ');
}

function prettyJson(value: unknown): string | null {
  if (value == null) return null;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function titleCase(value: string): string {
  return value
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function renderContentBlock(block: ToolCallContentBlock): ContentBlockView {
  if ('text' in block && typeof block.text === 'string') {
    return { label: titleCase(block.type), body: block.text };
  }
  if ('diff' in block && typeof block.diff === 'string') {
    return { label: titleCase(block.type), body: block.diff };
  }
  const fallback =
    formatCommand(block) ??
    pickValue(block, ['path', 'uri', 'url', 'content', 'stdout', 'stderr', 'output']) ??
    prettyJson(block) ??
    '';
  return { label: titleCase(block.type), body: fallback };
}

function optionClasses(kind: string): string {
  if (kind === 'allow_once' || kind === 'allow_always') {
    return 'bg-status-completed text-text-primary hover:bg-status-completed/90';
  }
  if (kind === 'reject_once' || kind === 'reject_always') {
    return 'bg-danger text-text-primary hover:bg-danger/90';
  }
  return '';
}

const toolCall = computed(() => props.request.toolCall);
const contextRows = computed<ContextRow[]>(() => {
  const payload = toolCall.value.rawInput;
  if (!payload) return [];
  const rows: ContextRow[] = [];
  const add = (label: string, value: string | null) => value && rows.push({ label, value });
  add('Command', formatCommand(payload));
  add('Working directory', pickValue(payload, ['cwd', 'workingDirectory', 'workdir']));
  add(
    'Target',
    pickValue(payload, ['path', 'filePath', 'targetPath', 'sourcePath', 'destinationPath', 'uri', 'url'])
  );
  add('Query', pickValue(payload, ['query', 'pattern']));
  const contentPreview = pickValue(payload, ['content', 'text']);
  add('Content preview', contentPreview ? truncate(contentPreview) : null);
  return rows;
});
const renderedBlocks = computed(() => (toolCall.value.content ?? []).map(renderContentBlock));
const rawInputJson = computed(() => prettyJson(toolCall.value.rawInput));
const rawOutputJson = computed(() => prettyJson(toolCall.value.rawOutput));
</script>

<template>
  <div class="entry-details flex flex-col gap-3 min-h-0" role="listitem">
    <div class="flex flex-col gap-1">
      <span class="text-body-sm font-semibold">{{ request.toolCall.title }}</span>
      <span class="text-caption text-text-secondary">{{ request.toolCall.kind }}</span>
    </div>

    <ScrollArea class="flex-1 min-h-0 [&>[data-radix-scroll-area-viewport]]:[overscroll-behavior:contain] pr-1">
    <div class="flex flex-col gap-3">
      <section v-if="contextRows.length">
        <h4 class="m-0 mb-2 text-title">Requested action</h4>
        <dl class="grid grid-cols-[minmax(110px,auto)_1fr] gap-y-2 gap-x-3 m-0">
          <template v-for="row in contextRows" :key="row.label">
            <dt class="font-semibold text-text-secondary">{{ row.label }}</dt>
            <dd class="m-0 min-w-0 break-words text-text-primary [overflow-wrap:anywhere]">{{ row.value }}</dd>
          </template>
        </dl>
      </section>

      <section v-if="request.toolCall.locations?.length" class="max-w-full rounded-[4px] border border-border-subtle bg-bg-surface px-3 py-2">
        <h4 class="m-0 mb-2 text-title">Locations</h4>
        <div v-for="(location, index) in request.toolCall.locations" :key="index" class="break-all py-0.5 font-mono text-caption text-text-primary">
          📁 {{ location.path }}
        </div>
      </section>

      <section v-if="renderedBlocks.length">
        <h4 class="m-0 mb-2 text-title">Tool content</h4>
        <div v-for="(block, index) in renderedBlocks" :key="`${block.label}-${index}`" class="min-w-0 [&+&]:mt-3">
          <div class="mb-1 text-caption font-semibold tracking-[0.04em] text-text-secondary">{{ block.label }}</div>
          <pre class="m-0 max-w-full overflow-auto whitespace-pre-wrap break-words rounded-[4px] border border-border-subtle bg-bg-surface p-3 font-mono text-caption leading-[1.4] [overflow-wrap:anywhere]">{{ block.body }}</pre>
        </div>
      </section>

      <Accordion v-if="rawInputJson || rawOutputJson" type="multiple">
        <AccordionItem v-if="rawInputJson" value="input" class="min-w-0 mt-3">
          <AccordionTrigger class="cursor-pointer font-semibold">Raw tool input</AccordionTrigger>
          <AccordionContent>
            <pre class="m-0 max-w-full overflow-auto whitespace-pre-wrap break-words rounded-[4px] border border-border-subtle bg-bg-surface p-3 font-mono text-caption leading-[1.4] [overflow-wrap:anywhere]">{{ rawInputJson }}</pre>
          </AccordionContent>
        </AccordionItem>

        <AccordionItem v-if="rawOutputJson" value="output" class="min-w-0 mt-3">
          <AccordionTrigger class="cursor-pointer font-semibold">Raw tool output</AccordionTrigger>
          <AccordionContent>
            <pre class="m-0 max-w-full overflow-auto whitespace-pre-wrap break-words rounded-[4px] border border-border-subtle bg-bg-surface p-3 font-mono text-caption leading-[1.4] [overflow-wrap:anywhere]">{{ rawOutputJson }}</pre>
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    </div>
    </ScrollArea>

    <div class="dialog-actions mt-1 flex flex-wrap gap-2 border-t border-border-subtle bg-bg-app pt-4">
      <Button
        v-for="option in request.options"
        :key="option.optionId"
        :class="[`option-${option.kind}`, 'flex-1 min-w-[120px] cursor-pointer rounded-[4px] border-none px-4 py-2.5 text-body-sm font-medium transition-[background] duration-150', optionClasses(option.kind)]"
        @click="emit('select', option.optionId)"
      >
        {{ option.name }}
      </Button>
      <Button variant="outline" class="cancel-btn flex-1 min-w-[120px] cursor-pointer rounded-[4px] border border-border-subtle bg-bg-app px-4 py-2.5 text-body-sm hover:bg-bg-surface" @click="emit('cancel')">Cancel</Button>
    </div>
  </div>
</template>
