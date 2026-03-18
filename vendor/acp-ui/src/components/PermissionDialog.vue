<script setup lang="ts">
import { computed } from 'vue';
import type { PermissionRequest } from '../lib/types';

const props = defineProps<{
  request: PermissionRequest;
  totalPending?: number;
}>();

const emit = defineEmits<{
  select: [optionId: string];
  cancel: [];
}>();

function handleSelect(optionId: string) {
  emit('select', optionId);
}

function handleCancel() {
  emit('cancel');
}

type ToolContextRow = {
  label: string;
  value: string;
};

type RenderedToolContent = {
  label: string;
  body: string;
};

function isPrimitive(value: unknown): value is string | number | boolean | bigint {
  return (
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    typeof value === 'bigint'
  );
}

function formatValue(value: unknown): string | null {
  if (typeof value === 'string') {
    return value;
  }

  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value);
  }

  if (Array.isArray(value) && value.every(isPrimitive)) {
    return value.map(String).join(', ');
  }

  return null;
}

function truncate(value: string, max = 240): string {
  const normalized = value.trim();
  if (normalized.length <= max) {
    return normalized;
  }

  return `${normalized.slice(0, max).trimEnd()}…`;
}

function getFirstString(payload: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const value = formatValue(payload[key]);
    if (value) {
      return value;
    }
  }

  return null;
}

function getStringArray(payload: Record<string, unknown>, keys: string[]): string[] {
  for (const key of keys) {
    const value = payload[key];
    if (Array.isArray(value)) {
      const items = value
        .map((entry) => (isPrimitive(entry) ? String(entry) : null))
        .filter((entry): entry is string => entry !== null);
      if (items.length > 0) {
        return items;
      }
    }
  }

  return [];
}

function formatCommand(payload: Record<string, unknown>): string | null {
  const command = getFirstString(payload, ['command', 'cmd', 'program']);
  const args = getStringArray(payload, ['args', 'argv']);

  if (!command && args.length === 0) {
    return null;
  }

  return [command, ...args].filter((part): part is string => Boolean(part)).join(' ');
}

function toPrettyJson(value: unknown): string | null {
  if (value == null) {
    return null;
  }

  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function renderToolContentBlock(block: { type: string; [key: string]: unknown }): RenderedToolContent {
  const typeLabel = block.type
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');

  if (typeof block.text === 'string') {
    return { label: typeLabel, body: block.text };
  }

  if (typeof block.diff === 'string') {
    return { label: typeLabel, body: block.diff };
  }

  const command = formatCommand(block);
  const value =
    command ??
    getFirstString(block, ['path', 'uri', 'url', 'content', 'stdout', 'stderr', 'output']) ??
    toPrettyJson(block) ??
    '';

  return {
    label: typeLabel,
    body: value,
  };
}

const toolCall = computed(() => props.request.toolCall);

const contextRows = computed<ToolContextRow[]>(() => {
  const payload = toolCall.value.rawInput;
  if (!payload) {
    return [];
  }

  const rows: ToolContextRow[] = [];
  const command = formatCommand(payload);
  if (command) {
    rows.push({ label: 'Command', value: command });
  }

  const cwd = getFirstString(payload, ['cwd', 'workingDirectory', 'workdir']);
  if (cwd) {
    rows.push({ label: 'Working directory', value: cwd });
  }

  const pathValue = getFirstString(payload, [
    'path',
    'filePath',
    'targetPath',
    'sourcePath',
    'destinationPath',
    'uri',
    'url',
  ]);
  if (pathValue) {
    rows.push({ label: 'Target', value: pathValue });
  }

  const query = getFirstString(payload, ['query', 'pattern']);
  if (query) {
    rows.push({ label: 'Query', value: query });
  }

  const contentPreview = getFirstString(payload, ['content', 'text']);
  if (contentPreview) {
    rows.push({ label: 'Content preview', value: truncate(contentPreview) });
  }

  return rows;
});

const renderedContentBlocks = computed<RenderedToolContent[]>(() =>
  (toolCall.value.content ?? []).map(renderToolContentBlock)
);

const rawInputJson = computed(() => toPrettyJson(toolCall.value.rawInput));
const rawOutputJson = computed(() => toPrettyJson(toolCall.value.rawOutput));
</script>

<template>
  <div class="permission-overlay">
    <div class="permission-dialog">
      <div class="dialog-header">
        <span class="icon">🔐</span>
        <h3>Permission Required</h3>
      </div>
      
      <div class="dialog-content">
        <div class="tool-info">
          <span class="tool-title">{{ request.toolCall.title }}</span>
          <span class="tool-kind">{{ request.toolCall.kind }}</span>
        </div>

        <p v-if="(totalPending ?? 0) > 1" class="queue-hint">
          Showing request 1 of {{ totalPending }}. The remaining requests will appear after you
          respond.
        </p>

        <div v-if="contextRows.length" class="context-section">
          <h4>Requested action</h4>
          <dl class="context-rows">
            <template v-for="row in contextRows" :key="row.label">
              <dt>{{ row.label }}</dt>
              <dd>{{ row.value }}</dd>
            </template>
          </dl>
        </div>

        <div v-if="request.toolCall.locations?.length" class="locations">
          <h4>Locations</h4>
          <div 
            v-for="(loc, index) in request.toolCall.locations" 
            :key="index"
            class="location"
          >
            📁 {{ loc.path }}
          </div>
        </div>

        <div v-if="renderedContentBlocks.length" class="context-section">
          <h4>Tool content</h4>
          <div
            v-for="(block, index) in renderedContentBlocks"
            :key="`${block.label}-${index}`"
            class="content-block"
          >
            <div class="content-label">{{ block.label }}</div>
            <pre class="content-preview">{{ block.body }}</pre>
          </div>
        </div>

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
          @click="handleSelect(option.optionId)"
        >
          {{ option.name }}
        </button>
        <button class="cancel-btn" @click="handleCancel">
          Cancel
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.permission-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.permission-dialog {
  background: var(--bg-dialog, #fff);
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  max-width: 480px;
  width: 90%;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  background: var(--bg-header, #f5f5f5);
  border-bottom: 1px solid var(--border-color, #e0e0e0);
}

.dialog-header .icon {
  font-size: 1.5rem;
}

.dialog-header h3 {
  margin: 0;
  font-size: 1.1rem;
}

.dialog-content {
  padding: 1rem;
}

.tool-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.tool-title {
  font-weight: 600;
  font-size: 1rem;
}

.tool-kind {
  font-size: 0.875rem;
  color: var(--text-muted, #666);
  text-transform: capitalize;
}

.context-section {
  margin-top: 1rem;
}

.context-section h4,
.locations h4 {
  margin: 0 0 0.5rem;
  font-size: 0.9rem;
}

.context-rows {
  display: grid;
  grid-template-columns: minmax(110px, auto) 1fr;
  gap: 0.5rem 0.75rem;
  margin: 0;
}

.context-rows dt {
  font-weight: 600;
  color: var(--text-secondary, #555);
}

.context-rows dd {
  margin: 0;
  color: var(--text-primary, #333);
  word-break: break-word;
}

.locations {
  margin-top: 0.75rem;
  padding: 0.5rem 0.75rem;
  background: var(--bg-hover, #f5f5f5);
  border-radius: 4px;
  border: 1px solid var(--border-color, #e0e0e0);
}

.location {
  font-family: monospace;
  font-size: 0.8rem;
  padding: 0.125rem 0;
  color: var(--text-primary, #333);
  word-break: break-all;
}

.queue-hint {
  margin-top: 0.75rem;
  font-size: 0.85rem;
  color: var(--text-secondary, #555);
}

.content-block {
  margin-top: 0.75rem;
}

.content-label {
  margin-bottom: 0.25rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary, #555);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.content-preview,
.raw-payload pre {
  margin: 0;
  padding: 0.75rem;
  background: var(--bg-hover, #f5f5f5);
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 4px;
  font-family: monospace;
  font-size: 0.8rem;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-word;
}

.raw-payload {
  margin-top: 1rem;
}

.raw-payload summary {
  cursor: pointer;
  font-weight: 600;
}

.dialog-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding: 1rem;
  border-top: 1px solid var(--border-color, #e0e0e0);
  background: var(--bg-footer, #fafafa);
}

.option-btn {
  flex: 1;
  min-width: 120px;
  padding: 0.625rem 1rem;
  border: none;
  border-radius: 4px;
  font-size: 0.9rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}

.option-allow_once,
.option-allow_always {
  background: var(--bg-success, #28a745);
  color: white;
}

.option-allow_once:hover,
.option-allow_always:hover {
  background: var(--bg-success-hover, #218838);
}

.option-reject_once,
.option-reject_always {
  background: var(--bg-danger, #dc3545);
  color: white;
}

.option-reject_once:hover,
.option-reject_always:hover {
  background: var(--bg-danger-hover, #c82333);
}

.cancel-btn {
  flex: 1;
  min-width: 120px;
  padding: 0.625rem 1rem;
  border: 1px solid var(--border-color, #ccc);
  border-radius: 4px;
  background: var(--bg-button, #fff);
  font-size: 0.9rem;
  cursor: pointer;
}

.cancel-btn:hover {
  background: var(--bg-hover, #f0f0f0);
}
</style>
