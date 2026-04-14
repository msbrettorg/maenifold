<script setup lang="ts">
import { computed } from 'vue';
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
  <div class="permission-entry" role="listitem">
    <div class="tool-info">
      <span class="tool-title">{{ request.toolCall.title }}</span>
      <span class="tool-kind">{{ request.toolCall.kind }}</span>
    </div>

    <div class="entry-details">
      <section v-if="contextRows.length" class="section">
        <h4>Requested action</h4>
        <dl class="context-rows">
          <template v-for="row in contextRows" :key="row.label">
            <dt>{{ row.label }}</dt>
            <dd>{{ row.value }}</dd>
          </template>
        </dl>
      </section>

      <section v-if="request.toolCall.locations?.length" class="section locations">
        <h4>Locations</h4>
        <div v-for="(location, index) in request.toolCall.locations" :key="index" class="location">
          📁 {{ location.path }}
        </div>
      </section>

      <section v-if="renderedBlocks.length" class="section">
        <h4>Tool content</h4>
        <div v-for="(block, index) in renderedBlocks" :key="`${block.label}-${index}`" class="content-block">
          <div class="content-label">{{ block.label }}</div>
          <pre class="content-preview">{{ block.body }}</pre>
        </div>
      </section>

      <details v-if="rawInputJson" class="raw-payload">
        <summary>Raw tool input</summary>
        <pre>{{ rawInputJson }}</pre>
      </details>

      <details v-if="rawOutputJson" class="raw-payload">
        <summary>Raw tool output</summary>
        <pre>{{ rawOutputJson }}</pre>
      </details>
    </div>

    <div class="dialog-actions">
      <button
        v-for="option in request.options"
        :key="option.optionId"
        :class="['option-btn', `option-${option.kind}`]"
        @click="emit('select', option.optionId)"
      >
        {{ option.name }}
      </button>
      <button class="cancel-btn" @click="emit('cancel')">Cancel</button>
    </div>
  </div>
</template>

<style scoped>
.permission-entry { display: flex; flex-direction: column; gap: 0.75rem; min-height: 0; }
.entry-details { display: flex; flex-direction: column; gap: 0.75rem; flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding-right: 0.25rem; }
.tool-info { display: flex; flex-direction: column; gap: 0.25rem; }
.tool-title { font-size: var(--text-body-sm); font-weight: var(--weight-title); }
.tool-kind { color: var(--muted-foreground); font-size: var(--text-caption); text-transform: capitalize; }
.section h4 { margin: 0 0 0.5rem; font-size: var(--text-title); }
.context-rows { display: grid; grid-template-columns: minmax(110px, auto) 1fr; gap: 0.5rem 0.75rem; margin: 0; }
.context-rows dt { color: var(--muted-foreground); font-weight: var(--weight-title); }
.context-rows dd { margin: 0; min-width: 0; color: var(--foreground); word-break: break-word; overflow-wrap: anywhere; }
.locations,
.content-preview,
.raw-payload pre { background: var(--muted); border: 1px solid var(--border); border-radius: 4px; max-inline-size: 100%; }
.locations { padding: 0.5rem 0.75rem; }
.location { color: var(--foreground); font-family: var(--font-code); font-size: var(--text-caption); padding: 0.125rem 0; word-break: break-all; }
.content-block + .content-block,
.raw-payload { margin-top: 0.75rem; }
.content-block,
.raw-payload { min-width: 0; }
.content-label { margin-bottom: 0.25rem; color: var(--muted-foreground); font-size: var(--text-caption); font-weight: var(--weight-title); letter-spacing: 0.04em; }
.content-preview,
.raw-payload pre { margin: 0; padding: 0.75rem; font-family: var(--font-code); font-size: var(--text-caption); line-height: 1.4; white-space: pre-wrap; word-break: break-word; overflow-wrap: anywhere; overflow: auto; }
.raw-payload summary { cursor: pointer; font-weight: var(--weight-title); }
.dialog-actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.25rem; padding-top: 1rem; border-top: 1px solid var(--border); background: var(--background); }
.option-btn,
.cancel-btn { flex: 1; min-width: 120px; padding: 0.625rem 1rem; border-radius: 4px; font-size: var(--text-body-sm); cursor: pointer; }
.option-btn { border: none; font-weight: var(--weight-display); transition: background 0.15s; }
.option-allow_once,
.option-allow_always { background: var(--color-status-completed); color: var(--primary-foreground); }
.option-allow_once:hover,
.option-allow_always:hover { background: color-mix(in srgb, var(--color-status-completed) 88%, var(--background)); }
.option-reject_once,
.option-reject_always { background: var(--destructive); color: var(--primary-foreground); }
.option-reject_once:hover,
.option-reject_always:hover { background: color-mix(in srgb, var(--destructive) 88%, var(--background)); }
.cancel-btn { background: var(--background); border: 1px solid var(--border); }
.cancel-btn:hover { background: var(--muted); }
</style>
