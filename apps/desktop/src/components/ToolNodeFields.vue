<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useMcpToolDiscovery } from '../composables/useMcpToolDiscovery';
import { resolveToolDefinition } from '../lib/tool-categories';
import type { McpDiscoveredTool } from '../lib/types';
import { usePipelineStore } from '../stores/pipeline';
import SchemaForm from './SchemaForm.vue';

interface ToolPickerSection {
  server: string;
  status: 'ok' | 'error';
  error?: string | null;
  tools: Array<McpDiscoveredTool & { namespacedName: string; displayName: string }>;
}

const props = withDefaults(defineProps<{
  active?: boolean;
  toolName: string;
  toolArgs: Record<string, unknown>;
}>(), {
  active: false,
});

const emit = defineEmits<{
  'update:toolName': [value: string];
  'update:toolArgs': [value: Record<string, unknown>];
  validation: [state: { valid: boolean }];
}>();

const pipelineStore = usePipelineStore();
const { serverResults, loading, error, refreshTools } = useMcpToolDiscovery();
const refreshing = ref(false);
const attemptedInitialLoad = ref(false);

const sections = computed<ToolPickerSection[]>(() =>
  serverResults.value.map((result) => ({
    server: result.server,
    status: result.status,
    error: result.error,
    tools: [...result.tools]
      .sort((left, right) => left.name.localeCompare(right.name))
      .map((tool) => ({
        ...tool,
        // Backend already returns names as "server.toolName" — do NOT re-prefix.
        namespacedName: tool.name,
        displayName: tool.name.startsWith(`${result.server}.`)
          ? tool.name.slice(result.server.length + 1)
          : tool.name,
      })),
  })),
);

const selectedDiscoveryTool = computed<McpDiscoveredTool | null>(() => {
  for (const section of sections.value) {
    const match = section.tools.find((tool) => tool.namespacedName === props.toolName);
    if (match) return match;
  }
  return null;
});

const selectedFallbackTool = computed(() =>
  resolveToolDefinition(pipelineStore.tools, props.toolName) ?? null,
);

const selectedToolSchema = computed(
  () => selectedDiscoveryTool.value?.inputSchema ?? selectedFallbackTool.value?.inputSchema ?? null,
);

const hasCatalog = computed(() => sections.value.length > 0);
const isBusy = computed(() => loading.value || refreshing.value);

async function refreshCatalog(): Promise<void> {
  refreshing.value = true;
  try {
    await Promise.allSettled([
      refreshTools(),
      pipelineStore.tools.length > 0 ? Promise.resolve(pipelineStore.tools) : pipelineStore.refreshTools(),
    ]);
  } finally {
    refreshing.value = false;
    attemptedInitialLoad.value = true;
  }
}

function selectTool(toolName: string): void {
  emit('update:toolName', toolName);
}

function updateToolName(event: Event): void {
  emit('update:toolName', (event.target as HTMLInputElement).value);
}

function onToolArgsChange(data: Record<string, unknown>): void {
  emit('update:toolArgs', data);
}

function onToolArgsValidation(state: { valid: boolean }): void {
  emit('validation', state);
}

watch(
  () => props.active,
  (active) => {
    if (!active || attemptedInitialLoad.value || hasCatalog.value) return;
    void refreshCatalog();
  },
  { immediate: true },
);
</script>

<template>
  <div class="tool-picker field-wide">
    <div class="tool-picker__header">
      <div>
        <span>Tool Picker</span>
        <p>Select an MCP tool by server, or enter a tool name manually.</p>
      </div>
      <button type="button" class="btn btn-ghost" :disabled="isBusy" @click="refreshCatalog">
        {{ isBusy ? 'Refreshing…' : 'Refresh Tools' }}
      </button>
    </div>

    <p v-if="error && !hasCatalog" class="tool-picker__status tool-picker__status--error">
      {{ error }}
    </p>

    <div v-if="hasCatalog" class="tool-picker__catalog">
      <section
        v-for="section in sections"
        :key="section.server"
        class="tool-picker__server panel"
        :class="{ 'tool-picker__server--error': section.status === 'error' }"
      >
        <header class="tool-picker__server-header">
          <div>
            <strong>{{ section.server }}</strong>
            <small>{{ section.tools.length }} tool{{ section.tools.length === 1 ? '' : 's' }}</small>
          </div>
          <span :class="['tool-picker__badge', `tool-picker__badge--${section.status}`]">
            {{ section.status }}
          </span>
        </header>
        <p v-if="section.error" class="tool-picker__status tool-picker__status--error">
          {{ section.error }}
          <span v-if="section.tools.length > 0">Showing last known tools.</span>
        </p>
        <p v-if="section.tools.length === 0" class="tool-picker__status">
          No tools available from this server.
        </p>
        <div v-else class="tool-picker__tool-list">
          <button
            v-for="tool in section.tools"
            :key="tool.namespacedName"
            type="button"
            class="tool-picker__tool"
            :class="{ 'is-selected': tool.namespacedName === toolName }"
            @click="selectTool(tool.namespacedName)"
          >
            <strong>{{ tool.displayName }}</strong>
            <span>{{ tool.description || 'No description provided.' }}</span>
          </button>
        </div>
      </section>
    </div>

    <label>
      <span>Tool Name</span>
      <input
        class="field-input"
        :value="toolName"
        type="text"
        placeholder="server.ToolName"
        @input="updateToolName"
      />
    </label>

    <div v-if="toolName" class="field-block">
      <span>Tool Args</span>
      <SchemaForm
        :schema="selectedToolSchema"
        :data="toolArgs"
        @change="onToolArgsChange"
        @validation="onToolArgsValidation"
      />
    </div>
  </div>
</template>

<style scoped>
.tool-picker,
.tool-picker__catalog,
.tool-picker__server,
.tool-picker__tool-list,
.field-block {
  display: grid;
  gap: 0.75rem;
}

.tool-picker__header,
.tool-picker__server-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.tool-picker__header p,
.tool-picker__status,
.tool-picker__tool span,
.tool-picker__server-header small,
.field-block {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.tool-picker__catalog {
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
}

.tool-picker__badge {
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  font-size: var(--text-xs);
  text-transform: uppercase;
}

.tool-picker__badge--ok {
  background: color-mix(in srgb, var(--color-status-completed) 18%, transparent);
  color: var(--color-status-completed);
}

.tool-picker__badge--error {
  background: color-mix(in srgb, var(--color-status-abandoned) 18%, transparent);
  color: var(--color-status-abandoned);
}

.tool-picker__server--error {
  border-color: color-mix(in srgb, var(--color-status-abandoned) 45%, var(--border-color));
}

.tool-picker__status--error {
  color: var(--color-status-abandoned);
}

.tool-picker__tool {
  display: grid;
  gap: 0.25rem;
  padding: 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  background: var(--bg-main);
  color: var(--text-primary);
  text-align: left;
}

.tool-picker__tool.is-selected {
  border-color: var(--border-focus);
  box-shadow: 0 0 0 1px var(--border-focus);
}
</style>
