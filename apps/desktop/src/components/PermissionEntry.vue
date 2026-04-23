<script setup lang="ts">
import { computed } from 'vue';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Button } from '@/components/ui/button';
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
    return 'bg-[var(--color-status-completed)] text-[var(--color-text-primary)] hover:bg-[color-mix(in_srgb,var(--color-status-completed)_88%,var(--color-bg-app))]';
  }
  if (kind === 'reject_once' || kind === 'reject_always') {
    return 'bg-[var(--color-danger)] text-[var(--color-text-primary)] hover:bg-[color-mix(in_srgb,var(--color-danger)_88%,var(--color-bg-app))]';
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
  <div class="flex flex-col gap-3 min-h-0" role="listitem">
    <div class="flex flex-col gap-1">
      <span class="text-[length:var(--text-body-sm)] font-[var(--weight-title)]">{{ request.toolCall.title }}</span>
      <span class="text-[var(--color-text-secondary)] text-[length:var(--text-caption)]">{{ request.toolCall.kind }}</span>
    </div>

    <div class="flex flex-col gap-3 flex-1 min-h-0 overflow-y-auto [overscroll-behavior:contain] pr-1">
      <section v-if="contextRows.length">
        <h4 class="m-0 mb-2 text-[length:var(--text-title)]">Requested action</h4>
        <dl class="grid grid-cols-[minmax(110px,auto)_1fr] gap-y-2 gap-x-3 m-0">
          <template v-for="row in contextRows" :key="row.label">
            <dt class="text-[var(--color-text-secondary)] font-[var(--weight-title)]">{{ row.label }}</dt>
            <dd class="m-0 min-w-0 text-[var(--color-text-primary)] break-words [overflow-wrap:anywhere]">{{ row.value }}</dd>
          </template>
        </dl>
      </section>

      <section v-if="request.toolCall.locations?.length" class="bg-[var(--color-bg-surface)] border border-[var(--color-border-subtle)] rounded-[4px] max-w-full py-2 px-3">
        <h4 class="m-0 mb-2 text-[length:var(--text-title)]">Locations</h4>
        <div v-for="(location, index) in request.toolCall.locations" :key="index" class="text-[var(--color-text-primary)] font-[var(--font-code)] text-[length:var(--text-caption)] py-0.5 break-all">
          📁 {{ location.path }}
        </div>
      </section>

      <section v-if="renderedBlocks.length">
        <h4 class="m-0 mb-2 text-[length:var(--text-title)]">Tool content</h4>
        <div v-for="(block, index) in renderedBlocks" :key="`${block.label}-${index}`" class="min-w-0 [&+&]:mt-3">
          <div class="mb-1 text-[var(--color-text-secondary)] text-[length:var(--text-caption)] font-[var(--weight-title)] tracking-[0.04em]">{{ block.label }}</div>
          <pre class="bg-[var(--color-bg-surface)] border border-[var(--color-border-subtle)] rounded-[4px] max-w-full m-0 p-3 font-[var(--font-code)] text-[length:var(--text-caption)] leading-[1.4] whitespace-pre-wrap break-words [overflow-wrap:anywhere] overflow-auto">{{ block.body }}</pre>
        </div>
      </section>

      <Accordion v-if="rawInputJson || rawOutputJson" type="multiple">
        <AccordionItem v-if="rawInputJson" value="input" class="min-w-0 mt-3">
          <AccordionTrigger class="font-[var(--weight-title)] cursor-pointer">Raw tool input</AccordionTrigger>
          <AccordionContent>
            <pre class="bg-[var(--color-bg-surface)] border border-[var(--color-border-subtle)] rounded-[4px] max-w-full m-0 p-3 font-[var(--font-code)] text-[length:var(--text-caption)] leading-[1.4] whitespace-pre-wrap break-words [overflow-wrap:anywhere] overflow-auto">{{ rawInputJson }}</pre>
          </AccordionContent>
        </AccordionItem>

        <AccordionItem v-if="rawOutputJson" value="output" class="min-w-0 mt-3">
          <AccordionTrigger class="font-[var(--weight-title)] cursor-pointer">Raw tool output</AccordionTrigger>
          <AccordionContent>
            <pre class="bg-[var(--color-bg-surface)] border border-[var(--color-border-subtle)] rounded-[4px] max-w-full m-0 p-3 font-[var(--font-code)] text-[length:var(--text-caption)] leading-[1.4] whitespace-pre-wrap break-words [overflow-wrap:anywhere] overflow-auto">{{ rawOutputJson }}</pre>
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    </div>

    <div class="flex flex-wrap gap-2 mt-1 pt-4 border-t border-[var(--color-border-subtle)] bg-[var(--color-bg-app)]">
      <Button
        v-for="option in request.options"
        :key="option.optionId"
        :class="['flex-1 min-w-[120px] py-2.5 px-4 rounded-[4px] text-[length:var(--text-body-sm)] cursor-pointer border-none font-[var(--weight-display)] transition-[background] duration-150', optionClasses(option.kind)]"
        @click="emit('select', option.optionId)"
      >
        {{ option.name }}
      </Button>
      <Button variant="outline" class="flex-1 min-w-[120px] py-2.5 px-4 rounded-[4px] text-[length:var(--text-body-sm)] cursor-pointer bg-[var(--color-bg-app)] border border-[var(--color-border-subtle)] hover:bg-[var(--color-bg-surface)]" @click="emit('cancel')">Cancel</Button>
    </div>
  </div>
</template>
