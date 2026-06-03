<script setup lang="ts">
// Design-mode node config form (T-600.4). Teleported into `#pipeline-overlay`.
// Opens when the design controller has an `editingNodeId`. Renders the per-type
// fields (required fields block Save; optional fields are exposed but never
// block) from the declarative field map, edits a deep-clone draft, and commits
// via `design.replaceNode`. JSON-typed fields (schemas / agentConfig / toolArgs)
// are edited as text and parsed on save; a parse error blocks Save.
import { computed, reactive, ref, toRaw, watch } from 'vue';
import { usePipelineDesignStore } from '../../../stores/pipelineDesign';
import { useDesignController } from './useDesignController';
import {
  deletePath,
  fieldsForType,
  getPath,
  setPath,
  type FieldDef,
} from '../../../lib/nodeFormFields';
import type { PipelineNode, PipelineNodeType } from '../../../lib/pipeline-types';

const design = usePipelineDesignStore();
const controller = useDesignController();

const draft = ref<Record<string, unknown>>({});
const jsonText = reactive<Record<string, string>>({});
const jsonError = reactive<Record<string, string | null>>({});
const formError = ref<string | null>(null);

const open = computed(() => controller.editingNodeId.value !== null);

const nodeType = computed<PipelineNodeType>(() => {
  const type = draft.value.type;
  if (type === 'prompt' || type === 'sequentialThinking' || type === 'tool' || type === 'workflow') {
    return type;
  }
  return 'workflow';
});

// Prompt nodes always expose the sessionRef fields (prompt config = prompt +
// agent + sessionRef); other types never have them.
const fields = computed(() => fieldsForType(nodeType.value, nodeType.value === 'prompt'));
const allFields = computed<FieldDef[]>(() => [...fields.value.required, ...fields.value.optional]);

function findNode(id: string): PipelineNode | undefined {
  return design.localDefinition?.nodes.find((node) => node.id === id);
}

function initJsonMirrors(): void {
  for (const key of Object.keys(jsonText)) delete jsonText[key];
  for (const key of Object.keys(jsonError)) delete jsonError[key];
  for (const field of allFields.value) {
    if (field.kind !== 'json') continue;
    const value = getPath(draft.value, field.path);
    jsonText[field.path] = value === undefined ? '' : JSON.stringify(value, null, 2);
    jsonError[field.path] = null;
  }
}

watch(
  () => controller.editingNodeId.value,
  (id) => {
    formError.value = null;
    if (!id) {
      draft.value = {};
      return;
    }
    const node = findNode(id);
    // `node` is a reactive store value; `structuredClone` throws on Vue proxies
    // in WebView2, so unwrap with `toRaw` before cloning.
    draft.value = node
      ? (structuredClone(toRaw(node)) as unknown as Record<string, unknown>)
      : {};
    initJsonMirrors();
  },
  { immediate: true }
);

// --- field accessors -------------------------------------------------------

function strVal(path: string): string {
  const value = getPath(draft.value, path);
  return value === undefined || value === null ? '' : String(value);
}

function boolVal(path: string): boolean {
  return Boolean(getPath(draft.value, path));
}

function listVal(path: string): string {
  const value = getPath(draft.value, path);
  return Array.isArray(value) ? value.join(', ') : '';
}

function setField(path: string, value: unknown): void {
  setPath(draft.value, path, value);
  draft.value = { ...draft.value };
}

function onText(path: string, event: Event): void {
  setField(path, (event.target as HTMLInputElement | HTMLTextAreaElement).value);
}

function onNumber(path: string, event: Event): void {
  const raw = (event.target as HTMLInputElement).value;
  setField(path, raw === '' ? undefined : Number(raw));
}

function onBool(path: string, event: Event): void {
  setField(path, (event.target as HTMLInputElement).checked);
}

function onSelect(path: string, event: Event): void {
  setField(path, (event.target as HTMLSelectElement).value || undefined);
}

function onList(path: string, event: Event): void {
  const parts = (event.target as HTMLInputElement).value
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);
  setField(path, parts);
}

function onJson(path: string, event: Event): void {
  const raw = (event.target as HTMLTextAreaElement).value;
  jsonText[path] = raw;
  if (!raw.trim()) {
    jsonError[path] = null;
    return;
  }
  try {
    JSON.parse(raw);
    jsonError[path] = null;
  } catch (error) {
    jsonError[path] = error instanceof Error ? error.message : 'Invalid JSON';
  }
}

// --- save ------------------------------------------------------------------

function buildResult(): Record<string, unknown> | null {
  // `draft.value` is a reactive ref value; unwrap before cloning (see above).
  const result = structuredClone(toRaw(draft.value)) as Record<string, unknown>;
  // Commit JSON fields (text → parsed value or removed).
  for (const field of allFields.value) {
    if (field.kind !== 'json') continue;
    const raw = jsonText[field.path] ?? '';
    if (!raw.trim()) {
      deletePath(result, field.path);
      continue;
    }
    try {
      setPath(result, field.path, JSON.parse(raw));
    } catch {
      formError.value = `Invalid JSON in "${field.label}".`;
      return null;
    }
  }
  // Drop empty optional scalars / arrays so the saved definition stays terse.
  for (const field of fields.value.optional) {
    if (field.kind === 'json') continue;
    const value = getPath(result, field.path);
    const isEmpty =
      value === undefined ||
      value === '' ||
      (Array.isArray(value) && value.length === 0);
    if (isEmpty) deletePath(result, field.path);
  }
  // Drop an entirely-empty sessionRef object (prompt nodes).
  const sessionRef = result.sessionRef;
  if (
    sessionRef &&
    typeof sessionRef === 'object' &&
    !((sessionRef as Record<string, unknown>).agent || (sessionRef as Record<string, unknown>).sessionId)
  ) {
    delete result.sessionRef;
  }
  return result;
}

function requiredMissing(result: Record<string, unknown>): string | null {
  for (const field of fields.value.required) {
    const value = getPath(result, field.path);
    if (typeof value !== 'string' || !value.trim()) {
      return `"${field.label}" is required.`;
    }
  }
  return null;
}

function save(): void {
  const id = controller.editingNodeId.value;
  if (!id) return;
  formError.value = null;
  const result = buildResult();
  if (!result) return;
  const missing = requiredMissing(result);
  if (missing) {
    formError.value = missing;
    return;
  }
  design.replaceNode(id, result as unknown as PipelineNode);
  controller.closeNodeForm();
}

function cancel(): void {
  controller.closeNodeForm();
}

const headingType = computed(() =>
  nodeType.value === 'sequentialThinking' ? 'thinking' : nodeType.value
);
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open" class="form-backdrop" @click.self="cancel">
      <section class="form-panel" role="dialog" aria-label="Edit node">
        <header class="form-head">
          <h2>Edit {{ headingType }} node</h2>
          <code class="form-id">{{ controller.editingNodeId.value }}</code>
        </header>

        <div class="form-body">
          <fieldset>
            <legend>Required</legend>
            <label v-for="field in fields.required" :key="field.path" class="field">
              <span class="field-label">{{ field.label }} *</span>
              <textarea
                v-if="field.kind === 'textarea'"
                class="field-input"
                rows="3"
                :value="strVal(field.path)"
                @input="onText(field.path, $event)"
              ></textarea>
              <input
                v-else
                class="field-input"
                type="text"
                :value="strVal(field.path)"
                @input="onText(field.path, $event)"
              />
            </label>
          </fieldset>

          <fieldset>
            <legend>Optional</legend>
            <label v-for="field in fields.optional" :key="field.path" class="field">
              <span class="field-label">{{ field.label }}</span>

              <textarea
                v-if="field.kind === 'textarea'"
                class="field-input"
                rows="3"
                :value="strVal(field.path)"
                @input="onText(field.path, $event)"
              ></textarea>

              <template v-else-if="field.kind === 'json'">
                <textarea
                  class="field-input mono"
                  rows="3"
                  :value="jsonText[field.path]"
                  @input="onJson(field.path, $event)"
                ></textarea>
                <span v-if="jsonError[field.path]" class="field-error">{{ jsonError[field.path] }}</span>
              </template>

              <select
                v-else-if="field.kind === 'select'"
                class="field-input"
                :value="strVal(field.path)"
                @change="onSelect(field.path, $event)"
              >
                <option value="">—</option>
                <option v-for="opt in field.options" :key="opt" :value="opt">{{ opt }}</option>
              </select>

              <input
                v-else-if="field.kind === 'boolean'"
                type="checkbox"
                class="field-check"
                :checked="boolVal(field.path)"
                @change="onBool(field.path, $event)"
              />

              <input
                v-else-if="field.kind === 'number'"
                type="number"
                class="field-input"
                :value="strVal(field.path)"
                @input="onNumber(field.path, $event)"
              />

              <input
                v-else-if="field.kind === 'stringlist'"
                type="text"
                class="field-input"
                placeholder="comma,separated"
                :value="listVal(field.path)"
                @input="onList(field.path, $event)"
              />

              <input
                v-else
                type="text"
                class="field-input"
                :value="strVal(field.path)"
                @input="onText(field.path, $event)"
              />
            </label>
          </fieldset>
        </div>

        <footer class="form-foot">
          <span v-if="formError" class="form-error" role="alert">{{ formError }}</span>
          <span class="form-spacer" />
          <button type="button" class="form-btn" @click="cancel">Cancel</button>
          <button type="button" class="form-btn primary" @click="save">Apply</button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.form-backdrop {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--bg-main) 30%, transparent);
  z-index: 20;
}

.form-panel {
  display: flex;
  flex-direction: column;
  width: min(540px, 92%);
  max-height: 88%;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-main);
  box-shadow: 0 14px 40px rgba(0, 0, 0, 0.32);
  overflow: hidden;
}

.form-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.7rem 0.9rem;
  border-bottom: 1px solid var(--border-color);
}

.form-head h2 {
  margin: 0;
  font-size: 0.92rem;
  color: var(--text-primary);
  text-transform: capitalize;
}

.form-id {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.form-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.7rem 0.9rem;
}

fieldset {
  margin: 0 0 0.8rem;
  padding: 0.5rem 0.6rem 0.7rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

legend {
  padding: 0 0.35rem;
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-secondary);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  margin-bottom: 0.55rem;
}

.field-label {
  font-size: 0.74rem;
  color: var(--text-secondary);
}

.field-input {
  width: 100%;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  outline: none;
}

.field-input.mono {
  font-family: var(--font-mono, monospace);
  font-size: 0.76rem;
}

.field-check {
  align-self: flex-start;
}

.field-error {
  font-size: 0.7rem;
  color: var(--bg-danger, #d9534f);
}

.form-foot {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 0.9rem;
  border-top: 1px solid var(--border-color);
}

.form-spacer {
  flex: 1;
}

.form-error {
  font-size: 0.74rem;
  color: var(--bg-danger, #d9534f);
}

.form-btn {
  padding: 0.4rem 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}

.form-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.form-btn.primary {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 40%, transparent);
}
</style>
