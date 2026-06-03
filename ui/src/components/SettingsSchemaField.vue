<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ChevronDown, ChevronRight, Plus, RotateCcw, Save, Trash2 } from '@lucide/vue';

import {
  formatSchemaLabel,
  isPlainObject,
  pathToId,
  resolveSchema,
  stableStringify,
  toJsonText,
  type JsonSchemaNode,
} from '../lib/settingsSchema';

interface Props {
  path: string[];
  fieldKey: string;
  schema: JsonSchemaNode;
  value: unknown;
  removable?: boolean;
  forceJson?: boolean;
}

interface ObjectEntry {
  key: string;
  schema: JsonSchemaNode;
  value: unknown;
  removable: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  removable: false,
  forceJson: false,
});

const emit = defineEmits<{
  (e: 'save', path: string[], value: unknown): void;
  (e: 'remove', path: string[]): void;
}>();

function hasNonEmptyProperties(schema: JsonSchemaNode): boolean {
  return isPlainObject(schema.properties) && Object.keys(schema.properties).length > 0;
}

function declaresObjectShape(schema: JsonSchemaNode): boolean {
  return hasNonEmptyProperties(schema) || schema.additionalProperties === true || isPlainObject(schema.additionalProperties);
}

const textDraft = ref('');
const numberDraft = ref('');
const booleanDraft = ref(false);
const arrayDraft = ref<string[]>([]);
const arrayEntry = ref('');
const jsonDraft = ref('null');
const newEntryKey = ref('');
const newEntryValue = ref('""');
const addEntryError = ref<string | null>(null);

const resolvedSchema = computed<JsonSchemaNode>(() => resolveSchema(props.schema, props.value));

const label = computed(() => {
  const title = resolvedSchema.value.title;
  return typeof title === 'string' && title.trim() ? title : formatSchemaLabel(props.fieldKey);
});

const description = computed(() => {
  const details = resolvedSchema.value.description;
  return typeof details === 'string' ? details : '';
});

const fieldPath = computed(() => pathToId(props.path));

const schemaBaseType = computed(() => {
  if (props.forceJson) {
    return 'json';
  }

  if (resolvedSchema.value.oneOf || resolvedSchema.value.anyOf || resolvedSchema.value.allOf) {
    return 'json';
  }

  if (typeof resolvedSchema.value.type === 'string') {
    return resolvedSchema.value.type;
  }

  if (Array.isArray(resolvedSchema.value.type)) {
    // Nullable shape from backend e.g. ["string", "null"] - pick the non-null member.
    const nonNull = resolvedSchema.value.type.find((t) => t !== 'null');
    if (typeof nonNull === 'string') {
      return nonNull;
    }
  }

  if (declaresObjectShape(resolvedSchema.value)) {
    return 'object';
  }

  if (resolvedSchema.value.items) {
    return 'array';
  }

  return 'json';
});

const fieldType = computed(() => {
  if (schemaBaseType.value === 'object' && !declaresObjectShape(resolvedSchema.value)) {
    return 'json';
  }

  if (schemaBaseType.value === 'array' && resolvedSchema.value.items?.type !== 'string') {
    return 'json';
  }

  return schemaBaseType.value;
});

const rendersObjectGroup = computed(() => fieldType.value === 'object' && declaresObjectShape(resolvedSchema.value));

const enumOptions = computed<string[] | null>(() => {
  const e = resolvedSchema.value.enum;
  if (Array.isArray(e) && e.length > 0 && e.every((v) => typeof v === 'string')) {
    return e as string[];
  }
  return null;
});

const typeLabel = computed(() => {
  if (fieldType.value === 'array') {
    return 'string[]';
  }

  return fieldType.value;
});

const minimum = computed(() =>
  typeof resolvedSchema.value.minimum === 'number' ? resolvedSchema.value.minimum : undefined
);

const objectValue = computed<Record<string, unknown>>(() =>
  isPlainObject(props.value) ? props.value : {}
);

const definedProperties = computed<Record<string, JsonSchemaNode>>(
  () => resolvedSchema.value.properties ?? {}
);

const additionalSchema = computed<JsonSchemaNode | null>(() => {
  const additional = resolvedSchema.value.additionalProperties;
  return isPlainObject(additional) ? (additional as JsonSchemaNode) : null;
});

const supportsAdditionalProperties = computed(() => {
  const additional = resolvedSchema.value.additionalProperties;
  return additional === true || isPlainObject(additional);
});

const objectEntries = computed<ObjectEntry[]>(() => {
  const current = objectValue.value;
  const properties = definedProperties.value;
  const entries: ObjectEntry[] = Object.keys(properties).map((key) => ({
    key,
    schema: properties[key],
    value: current[key],
    removable: false,
  }));

  for (const key of Object.keys(current)) {
    if (key in properties) {
      continue;
    }

    entries.push({
      key,
      schema: additionalSchema.value ?? {},
      value: current[key],
      removable: true,
    });
  }

  return entries;
});

const numberValidation = computed(() => {
  if (fieldType.value !== 'integer' && fieldType.value !== 'number') {
    return { valid: true, value: props.value, message: '' };
  }

  const trimmed = numberDraft.value.trim();
  if (!trimmed) {
    return { valid: false, value: null, message: 'Enter a number before saving.' };
  }

  const parsed = Number(trimmed);
  if (Number.isNaN(parsed)) {
    return { valid: false, value: null, message: 'Enter a valid number.' };
  }

  if (fieldType.value === 'integer' && !Number.isInteger(parsed)) {
    return { valid: false, value: null, message: 'Enter a whole number.' };
  }

  if (
    typeof resolvedSchema.value.minimum === 'number' &&
    parsed < resolvedSchema.value.minimum
  ) {
    return {
      valid: false,
      value: null,
      message: `Value must be at least ${resolvedSchema.value.minimum}.`,
    };
  }

  return { valid: true, value: parsed, message: '' };
});

const jsonValidation = computed(() => {
  if (fieldType.value !== 'json') {
    return { valid: true, value: props.value, message: '' };
  }

  try {
    return {
      valid: true,
      value: JSON.parse(jsonDraft.value),
      message: '',
    };
  } catch (error) {
    return {
      valid: false,
      value: null,
      message: error instanceof Error ? error.message : String(error),
    };
  }
});

const validationMessage = computed(() => {
  if (fieldType.value === 'integer' || fieldType.value === 'number') {
    return numberValidation.value.message;
  }

  if (fieldType.value === 'json') {
    return jsonValidation.value.message;
  }

  return '';
});

const parsedLeafValue = computed(() => {
  switch (fieldType.value) {
    case 'string':
      return textDraft.value;
    case 'integer':
    case 'number':
      return numberValidation.value.value;
    case 'boolean':
      return booleanDraft.value;
    case 'array':
      return arrayDraft.value.slice();
    case 'json':
      return jsonValidation.value.value;
    default:
      return props.value;
  }
});

const isDirty = computed(() => {
  switch (fieldType.value) {
    case 'string':
      return textDraft.value !== (typeof props.value === 'string' ? props.value : '');
    case 'integer':
    case 'number':
      return numberDraft.value !== (typeof props.value === 'number' ? String(props.value) : '');
    case 'boolean':
      return booleanDraft.value !== Boolean(props.value);
    case 'array':
      return stableStringify(arrayDraft.value) !== stableStringify(Array.isArray(props.value) ? props.value : []);
    case 'json':
      return jsonDraft.value !== toJsonText(props.value);
    default:
      return false;
  }
});

const canSave = computed(() => isDirty.value && !validationMessage.value);

function syncDraftFromValue(): void {
  switch (fieldType.value) {
    case 'string':
      textDraft.value = typeof props.value === 'string' ? props.value : '';
      break;
    case 'integer':
    case 'number':
      numberDraft.value = typeof props.value === 'number' ? String(props.value) : '';
      break;
    case 'boolean':
      booleanDraft.value = Boolean(props.value);
      break;
    case 'array':
      arrayDraft.value = Array.isArray(props.value)
        ? props.value.filter((entry): entry is string => typeof entry === 'string')
        : [];
      arrayEntry.value = '';
      break;
    case 'json':
      jsonDraft.value = toJsonText(props.value);
      break;
    default:
      break;
  }
}

watch(
  () => [fieldType.value, props.value] as const,
  () => {
    syncDraftFromValue();
  },
  { deep: true, immediate: true }
);

function emitSave(): void {
  if (!canSave.value) {
    return;
  }

  emit('save', props.path, parsedLeafValue.value);
}

function emitRemove(): void {
  emit('remove', props.path);
}

function resetDraft(): void {
  syncDraftFromValue();
}

function addArrayValue(): void {
  const next = arrayEntry.value.trim();
  if (!next) {
    return;
  }

  arrayDraft.value = [...arrayDraft.value, next];
  arrayEntry.value = '';
}

function removeArrayValue(index: number): void {
  arrayDraft.value = arrayDraft.value.filter((_, currentIndex) => currentIndex !== index);
}

function handleAddEntry(): void {
  const key = newEntryKey.value.trim();
  if (!key) {
    addEntryError.value = 'Enter a key name before adding a new entry.';
    return;
  }

  if (key in objectValue.value) {
    addEntryError.value = `The key "${key}" already exists.`;
    return;
  }

  try {
    const parsed = JSON.parse(newEntryValue.value);
    emit('save', [...props.path, key], parsed);
    newEntryKey.value = '';
    newEntryValue.value = '""';
    addEntryError.value = null;
  } catch (error) {
    addEntryError.value = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <details v-if="rendersObjectGroup" class="settings-group" :open="path.length <= 1">
    <summary class="settings-group-summary">
      <div class="field-copy">
        <div class="field-title-row">
          <span class="field-label">{{ label }}</span>
          <span class="field-type">{{ typeLabel }}</span>
          <code class="field-path">{{ fieldPath }}</code>
        </div>
        <p v-if="description" class="field-description">{{ description }}</p>
      </div>

      <div class="group-summary-actions">
        <button
          v-if="removable"
          type="button"
          class="icon-button danger"
          @click.stop.prevent="emitRemove"
        >
          <Trash2 :size="14" :stroke-width="1.9" />
          <span>Remove</span>
        </button>

        <span class="group-chevron" aria-hidden="true">
          <ChevronRight class="chevron chevron-collapsed" :size="18" :stroke-width="2" />
          <ChevronDown class="chevron chevron-open" :size="18" :stroke-width="2" />
        </span>
      </div>
    </summary>

    <div class="settings-group-body">
      <p v-if="objectEntries.length === 0" class="empty-state">
        No keys loaded for this object yet.
      </p>

      <SettingsSchemaField
        v-for="entry in objectEntries"
        :key="`${fieldPath}.${entry.key}`"
        :path="[...path, entry.key]"
        :field-key="entry.key"
        :schema="entry.schema"
        :value="entry.value"
        :removable="entry.removable"
        @save="(nextPath, nextValue) => emit('save', nextPath, nextValue)"
        @remove="(nextPath) => emit('remove', nextPath)"
      />

      <div v-if="supportsAdditionalProperties" class="add-entry-card">
        <div class="add-entry-head">
          <h4 class="add-entry-title">Add entry</h4>
          <p class="add-entry-copy">
            Save a new additional-properties key by supplying a JSON value.
          </p>
        </div>

        <div class="add-entry-grid">
          <label class="add-entry-field">
            <span>Key</span>
            <input
              v-model="newEntryKey"
              type="text"
              class="input-control"
              :placeholder="`${fieldKey}Key`"
            />
          </label>

          <label class="add-entry-field add-entry-field--wide">
            <span>JSON value</span>
            <textarea
              v-model="newEntryValue"
              class="textarea-control textarea-control--small"
              rows="3"
              spellcheck="false"
            />
          </label>
        </div>

        <p v-if="addEntryError" class="field-error">{{ addEntryError }}</p>

        <div class="field-actions">
          <button type="button" class="icon-button" @click="handleAddEntry">
            <Plus :size="14" :stroke-width="1.9" />
            <span>Add key</span>
          </button>
        </div>
      </div>
    </div>
  </details>

  <div v-else class="field-row" :class="{ 'field-row--dirty': isDirty }">
    <div class="field-copy">
      <div class="field-title-row">
        <span class="field-label">{{ label }}</span>
        <span class="field-type">{{ typeLabel }}</span>
        <code class="field-path">{{ fieldPath }}</code>
      </div>
      <p v-if="description" class="field-description">{{ description }}</p>
    </div>

    <div class="field-editor">
      <select
        v-if="fieldType === 'string' && enumOptions"
        v-model="textDraft"
        class="input-control"
      >
        <option v-for="option in enumOptions" :key="option" :value="option">
          {{ option }}
        </option>
      </select>

      <input
        v-else-if="fieldType === 'string'"
        v-model="textDraft"
        type="text"
        class="input-control"
      />

      <input
        v-else-if="fieldType === 'integer' || fieldType === 'number'"
        v-model="numberDraft"
        type="number"
        class="input-control"
        :min="minimum"
        :step="fieldType === 'integer' ? 1 : 'any'"
      />

      <label v-else-if="fieldType === 'boolean'" class="toggle-switch">
        <input v-model="booleanDraft" type="checkbox" />
        <span class="toggle-track">
          <span class="toggle-thumb"></span>
        </span>
        <span class="toggle-state">{{ booleanDraft ? 'On' : 'Off' }}</span>
      </label>

      <div v-else-if="fieldType === 'array'" class="array-editor">
        <div class="chip-list">
          <span v-if="arrayDraft.length === 0" class="empty-chip">No items yet.</span>
          <span v-for="(item, index) in arrayDraft" :key="`${fieldPath}-${index}`" class="chip">
            <span>{{ item }}</span>
            <button
              type="button"
              class="chip-remove"
              :aria-label="`Remove ${item}`"
              @click="removeArrayValue(index)"
            >
              ×
            </button>
          </span>
        </div>

        <div class="array-compose">
          <input
            v-model="arrayEntry"
            type="text"
            class="input-control"
            :placeholder="`Add ${label}`"
            @keydown.enter.prevent="addArrayValue"
          />
          <button type="button" class="icon-button" @click="addArrayValue">
            <Plus :size="14" :stroke-width="1.9" />
            <span>Add</span>
          </button>
        </div>
      </div>

      <textarea
        v-else
        v-model="jsonDraft"
        class="textarea-control"
        rows="6"
        spellcheck="false"
      />

      <p v-if="isDirty && validationMessage" class="field-error">{{ validationMessage }}</p>

      <div class="field-actions">
        <button type="button" class="icon-button primary" :disabled="!canSave" @click="emitSave">
          <Save :size="14" :stroke-width="1.9" />
          <span>Save</span>
        </button>
        <button type="button" class="icon-button" :disabled="!isDirty" @click="resetDraft">
          <RotateCcw :size="14" :stroke-width="1.9" />
          <span>Reset</span>
        </button>
        <button v-if="removable" type="button" class="icon-button danger" @click="emitRemove">
          <Trash2 :size="14" :stroke-width="1.9" />
          <span>Remove</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.field-row {
  display: grid;
  grid-template-columns: minmax(0, 260px) minmax(0, 1fr);
  gap: 1rem;
  align-items: start;
  padding: 0.95rem 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
}

.field-row:last-child {
  border-bottom: none;
}

.field-row--dirty {
  background: color-mix(in srgb, var(--text-accent) 4%, transparent);
}

.field-copy {
  min-width: 0;
}

.field-title-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem;
}

.field-label {
  font-weight: 600;
  color: var(--text-primary);
}

.field-type,
.field-path {
  display: inline-flex;
  align-items: center;
  padding: 0.06rem 0.4rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
  font-size: 0.7rem;
  line-height: 1.4;
}

.field-type {
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.field-path {
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  color: var(--text-secondary);
}

.field-description {
  margin: 0.35rem 0 0;
  color: var(--text-secondary);
  font-size: 0.84rem;
  line-height: 1.5;
}

.field-editor {
  min-width: 0;
}

.input-control,
.textarea-control {
  width: 100%;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-sidebar) 65%, var(--bg-main));
  color: var(--text-primary);
  font: inherit;
  transition: border-color 0.12s ease, box-shadow 0.12s ease;
}

.input-control {
  min-height: 40px;
  padding: 0.55rem 0.7rem;
}

.textarea-control {
  min-height: 132px;
  padding: 0.7rem 0.75rem;
  resize: vertical;
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.84rem;
}

.textarea-control--small {
  min-height: 90px;
}

.input-control:focus,
.textarea-control:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--text-accent) 60%, var(--border-color));
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--text-accent) 12%, transparent);
}

.toggle-switch {
  display: inline-flex;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
}

.toggle-switch input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.toggle-track {
  position: relative;
  display: inline-flex;
  align-items: center;
  width: 48px;
  height: 28px;
  padding: 3px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 75%, var(--bg-main));
  transition: background 0.12s ease, border-color 0.12s ease;
}

.toggle-thumb {
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--text-muted);
  transition: transform 0.12s ease, background 0.12s ease;
}

.toggle-switch input:checked + .toggle-track {
  border-color: color-mix(in srgb, var(--text-accent) 55%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 24%, var(--bg-main));
}

.toggle-switch input:checked + .toggle-track .toggle-thumb {
  transform: translateX(20px);
  background: var(--text-accent);
}

.toggle-state {
  font-size: 0.84rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.array-editor {
  display: grid;
  gap: 0.7rem;
}

.chip-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.24rem 0.55rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 60%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.8rem;
}

.chip-remove {
  border: none;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  cursor: pointer;
}

.chip-remove:hover {
  color: var(--bg-danger);
}

.empty-chip,
.empty-state {
  color: var(--text-muted);
  font-size: 0.82rem;
}

.array-compose {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.6rem;
}

.field-error {
  margin: 0.55rem 0 0;
  color: var(--bg-danger);
  font-size: 0.8rem;
  line-height: 1.45;
}

.field-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  margin-top: 0.75rem;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  gap: 0.38rem;
  padding: 0.45rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 9px;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.icon-button:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.icon-button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.icon-button.primary {
  border-color: color-mix(in srgb, var(--text-accent) 55%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 16%, var(--bg-main));
  color: var(--text-accent);
}

.icon-button.primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 22%, var(--bg-main));
}

.icon-button.danger {
  border-color: color-mix(in srgb, var(--bg-danger) 45%, var(--border-color));
  color: var(--bg-danger);
}

.icon-button.danger:hover:not(:disabled) {
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
}

.settings-group {
  margin-top: 0.9rem;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: color-mix(in srgb, var(--bg-sidebar) 40%, var(--bg-main));
  overflow: hidden;
}

.settings-group-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  list-style: none;
  padding: 0.9rem 1rem;
  cursor: pointer;
}

.settings-group-summary::-webkit-details-marker {
  display: none;
}

.settings-group-body {
  padding: 0 1rem 1rem;
}

.group-summary-actions {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
}

.group-chevron {
  display: inline-flex;
  align-items: center;
  color: var(--text-muted);
}

.chevron-open {
  display: none;
}

.settings-group[open] .chevron-open {
  display: inline-flex;
}

.settings-group[open] .chevron-collapsed {
  display: none;
}

.add-entry-card {
  margin-top: 1rem;
  padding: 0.95rem;
  border: 1px dashed color-mix(in srgb, var(--text-accent) 28%, var(--border-color));
  border-radius: 12px;
  background: color-mix(in srgb, var(--text-accent) 4%, transparent);
}

.add-entry-head {
  margin-bottom: 0.75rem;
}

.add-entry-title {
  margin: 0;
  font-size: 0.95rem;
  color: var(--text-primary);
}

.add-entry-copy {
  margin: 0.25rem 0 0;
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.add-entry-grid {
  display: grid;
  grid-template-columns: minmax(0, 180px) minmax(0, 1fr);
  gap: 0.8rem;
}

.add-entry-field {
  display: grid;
  gap: 0.35rem;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.add-entry-field--wide {
  min-width: 0;
}

@media (max-width: 900px) {
  .field-row {
    grid-template-columns: 1fr;
  }

  .add-entry-grid,
  .array-compose {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .settings-group-summary {
    flex-direction: column;
    align-items: flex-start;
  }

  .group-summary-actions {
    width: 100%;
    justify-content: space-between;
  }
}
</style>
