<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { AlertTriangle, Info } from '@lucide/vue';

import SettingsSchemaField from './SettingsSchemaField.vue';
import { useAppSettingsStore, buildNestedPatch } from '../stores/appSettings';
import {
  formatSchemaLabel,
  isPlainObject,
  type JsonSchemaNode,
} from '../lib/settingsSchema';

interface RootEntry {
  key: string;
  schema: JsonSchemaNode;
  value: unknown;
}

interface AdvancedEntry {
  key: string;
  value: unknown;
}

const appSettingsStore = useAppSettingsStore();
const actionError = ref<string | null>(null);

const rootSchema = computed<JsonSchemaNode | null>(() => {
  return isPlainObject(appSettingsStore.schema)
    ? (appSettingsStore.schema as JsonSchemaNode)
    : null;
});

const schemaProperties = computed<Record<string, JsonSchemaNode>>(
  () => rootSchema.value?.properties ?? {}
);

const rootEntries = computed<RootEntry[]>(() => {
  const values = isPlainObject(appSettingsStore.values)
    ? (appSettingsStore.values as Record<string, unknown>)
    : {};

  return Object.keys(schemaProperties.value).map((key) => ({
    key,
    schema: schemaProperties.value[key],
    value: values[key],
  }));
});

const advancedEntries = computed<AdvancedEntry[]>(() => {
  const values = isPlainObject(appSettingsStore.values)
    ? (appSettingsStore.values as Record<string, unknown>)
    : {};

  return Object.keys(values)
    .filter((key) => !(key in schemaProperties.value))
    .map((key) => ({
      key,
      value: values[key],
    }));
});

const currentError = computed(() => actionError.value ?? appSettingsStore.error);

const daemonError = computed(() => {
  const message = currentError.value ?? '';
  return /MCP request failed|Failed to fetch|ECONNREFUSED|fetch failed|network/i.test(message);
});

const inlineError = computed(() =>
  currentError.value && !daemonError.value ? currentError.value : null
);

const schemaReady = computed(() => rootSchema.value !== null);

onMounted(() => {
  void Promise.allSettled([
    appSettingsStore.fetchSchema(),
    appSettingsStore.fetchConfig(),
  ]);
});

function commandPreview(path: string[], value: unknown): string {
  const envelope = {
    domain: 'maenifold.system',
    command: 'SetSettings',
    parameters: {
      settings: buildNestedPatch(path, value),
    },
  };

  return [
    'GatewayExecute',
    JSON.stringify(envelope, null, 2),
  ].join('\n');
}

async function handleSave(path: string[], value: unknown): Promise<void> {
  actionError.value = null;

  const preview = commandPreview(path, value);
  const confirmed = window.confirm(`${preview}\n\nSave this change?`);
  if (!confirmed) {
    return;
  }

  try {
    await appSettingsStore.setLeaf(path, value);
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error);
  }
}

async function handleRemove(path: string[]): Promise<void> {
  actionError.value = null;

  const preview = commandPreview(path, null);
  const confirmed = window.confirm(
    `${preview}\n\nRemove this key by sending null through SetSettings?`
  );
  if (!confirmed) {
    return;
  }

  try {
    await appSettingsStore.setLeaf(path, null);
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <div class="app-settings">
    <div class="schema-note">
      <Info :size="16" :stroke-width="1.9" />
      <div>
        <strong>Schema-driven renderer.</strong>
        Labels come from the fetched schema when available, and unknown root keys stay visible in
        <em>Advanced (additionalProperties)</em>.
      </div>
    </div>

    <div v-if="daemonError" class="soft-banner" role="status">
      <AlertTriangle :size="18" :stroke-width="1.9" />
      <div>
        <strong>Daemon unreachable.</strong>
        <span>{{ currentError }}</span>
      </div>
    </div>

    <div v-if="inlineError" class="inline-error" role="alert">
      <AlertTriangle :size="18" :stroke-width="1.9" />
      <div>
        <strong>Settings load error.</strong>
        <span>{{ inlineError }}</span>
      </div>
    </div>

    <div v-if="appSettingsStore.loading && !schemaReady" class="loading-state">
      Loading app settings schema and current values…
    </div>

    <div v-else-if="schemaReady" class="field-list">
      <SettingsSchemaField
        v-for="entry in rootEntries"
        :key="entry.key"
        :path="[entry.key]"
        :field-key="entry.key"
        :schema="entry.schema"
        :value="entry.value"
        @save="handleSave"
        @remove="handleRemove"
      />

      <details v-if="advancedEntries.length > 0" class="advanced-group" open>
        <summary class="advanced-summary">
          <span class="advanced-title">Advanced (additionalProperties)</span>
          <span class="advanced-count">{{ advancedEntries.length }}</span>
        </summary>

        <p class="advanced-copy">
          Keys returned by <code>GetConfig.settings</code> but absent from <code>GetSettingsSchema</code>.
        </p>

        <SettingsSchemaField
          v-for="entry in advancedEntries"
          :key="`advanced-${entry.key}`"
          :path="[entry.key]"
          :field-key="entry.key"
          :schema="{ title: formatSchemaLabel(entry.key) }"
          :value="entry.value"
          :force-json="true"
          @save="handleSave"
          @remove="handleRemove"
        />
      </details>
    </div>

    <div v-else class="empty-state">
      App settings could not be rendered because the schema is unavailable.
    </div>
  </div>
</template>

<style scoped>
.app-settings {
  padding: 1.15rem;
}

.schema-note,
.soft-banner,
.inline-error,
.loading-state,
.empty-state {
  display: flex;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
  border-radius: 14px;
  border: 1px solid var(--border-color);
}

.schema-note {
  margin-bottom: 1rem;
  background: color-mix(in srgb, var(--text-accent) 6%, transparent);
  color: var(--text-secondary);
}

.schema-note svg {
  flex-shrink: 0;
  color: var(--text-accent);
  margin-top: 0.1rem;
}

.soft-banner,
.inline-error {
  margin-bottom: 1rem;
  align-items: flex-start;
}

.soft-banner {
  border-color: color-mix(in srgb, var(--bg-warning) 45%, var(--border-color));
  background: color-mix(in srgb, var(--bg-warning) 18%, var(--bg-main));
  color: var(--text-primary);
}

.inline-error {
  border-color: color-mix(in srgb, var(--bg-danger) 45%, var(--border-color));
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  color: var(--text-primary);
}

.soft-banner svg,
.inline-error svg {
  flex-shrink: 0;
  margin-top: 0.1rem;
}

.soft-banner span,
.inline-error span {
  display: block;
  margin-top: 0.2rem;
  color: var(--text-secondary);
  line-height: 1.45;
}

.loading-state,
.empty-state {
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

.field-list {
  display: grid;
  gap: 0.9rem;
}

.advanced-group {
  margin-top: 0.25rem;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-sidebar) 35%, var(--bg-main));
  overflow: hidden;
}

.advanced-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.05rem;
  cursor: pointer;
  list-style: none;
}

.advanced-summary::-webkit-details-marker {
  display: none;
}

.advanced-title {
  font-weight: 600;
  color: var(--text-primary);
}

.advanced-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.8rem;
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
  color: var(--text-secondary);
  font-size: 0.74rem;
  font-weight: 600;
}

.advanced-copy {
  margin: 0;
  padding: 0 1.05rem 0.4rem;
  color: var(--text-secondary);
  font-size: 0.84rem;
  line-height: 1.45;
}

.advanced-copy code {
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.78rem;
}

@media (max-width: 640px) {
  .app-settings {
    padding: 1rem;
  }
}
</style>
