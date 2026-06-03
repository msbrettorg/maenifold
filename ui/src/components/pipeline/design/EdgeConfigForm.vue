<script setup lang="ts">
// Design-mode edge config form (T-600.4). Teleported into `#pipeline-overlay`.
// Opens when the design controller has an `editingEdgeIndex`. Edits a single
// edge: its `type` (success/always/conditional with path+equals), its data-flow
// `mode` (none/context/binding), and — for binding mode — a from→to mapping
// editor validated against the source node's `outputSchema` and the target
// node's `inputSchema`.
import { computed, ref, watch } from 'vue';
import { Plus, Trash2 } from '@lucide/vue';
import { usePipelineDesignStore } from '../../../stores/pipelineDesign';
import { useDesignController } from './useDesignController';
import type {
  EdgeConditionEquals,
  EdgeType,
  PipelineEdge,
  PipelineNode,
} from '../../../lib/pipeline-types';

const design = usePipelineDesignStore();
const controller = useDesignController();

type EqualsType = 'string' | 'number' | 'boolean';
interface BindingRow {
  from: string;
  to: string;
}

const open = computed(() => controller.editingEdgeIndex.value !== null);

const source = ref('');
const target = ref('');
const edgeType = ref<EdgeType>('success');
const path = ref('');
const equalsType = ref<EqualsType>('string');
const equalsRaw = ref('');
const mode = ref<'none' | 'context' | 'binding'>('none');
const rows = ref<BindingRow[]>([]);
const formError = ref<string | null>(null);

function currentEdge(): PipelineEdge | undefined {
  const index = controller.editingEdgeIndex.value;
  if (index === null) return undefined;
  return design.localDefinition?.edges[index];
}

function nodeById(id: string): PipelineNode | undefined {
  return design.localDefinition?.nodes.find((node) => node.id === id);
}

function detectEqualsType(value: EdgeConditionEquals | undefined): EqualsType {
  if (typeof value === 'number') return 'number';
  if (typeof value === 'boolean') return 'boolean';
  return 'string';
}

function rowsFromEdge(edge: PipelineEdge): BindingRow[] {
  const out: BindingRow[] = [];
  if (Array.isArray(edge.bindings)) {
    for (const binding of edge.bindings) out.push({ from: binding.from ?? '', to: binding.to ?? '' });
  }
  if (edge.mapping && typeof edge.mapping === 'object') {
    for (const [from, to] of Object.entries(edge.mapping)) {
      if (typeof to === 'string') out.push({ from, to });
    }
  }
  return out;
}

watch(
  () => controller.editingEdgeIndex.value,
  () => {
    formError.value = null;
    const edge = currentEdge();
    if (!edge) return;
    source.value = edge.source;
    target.value = edge.target;
    edgeType.value = edge.type ?? 'success';
    path.value = edge.path ?? '';
    equalsType.value = detectEqualsType(edge.equals);
    equalsRaw.value = edge.equals === undefined ? '' : String(edge.equals);
    mode.value = edge.mode ?? 'none';
    rows.value = rowsFromEdge(edge);
  },
  { immediate: true }
);

// --- schema-derived field suggestions --------------------------------------

function schemaKeys(node: PipelineNode | undefined, key: 'inputSchema' | 'outputSchema'): string[] {
  const schema = node ? (node as unknown as Record<string, unknown>)[key] : undefined;
  if (!schema || typeof schema !== 'object') return [];
  const props = (schema as Record<string, unknown>).properties;
  if (!props || typeof props !== 'object') return [];
  return Object.keys(props as Record<string, unknown>);
}

const sourceNode = computed(() => nodeById(source.value));
const targetNode = computed(() => nodeById(target.value));
const fromKeys = computed(() => schemaKeys(sourceNode.value, 'outputSchema'));
const toKeys = computed(() => schemaKeys(targetNode.value, 'inputSchema'));
const sourceHasOutput = computed(() => Boolean((sourceNode.value as unknown as Record<string, unknown> | undefined)?.outputSchema));
const targetHasInput = computed(() => Boolean((targetNode.value as unknown as Record<string, unknown> | undefined)?.inputSchema));

const bindingBlocked = computed(
  () => mode.value === 'binding' && (!sourceHasOutput.value || !targetHasInput.value)
);

function addRow(): void {
  rows.value = [...rows.value, { from: '', to: '' }];
}

function removeRow(index: number): void {
  rows.value = rows.value.filter((_, i) => i !== index);
}

function updateRow(index: number, key: keyof BindingRow, value: string): void {
  rows.value = rows.value.map((row, i) => (i === index ? { ...row, [key]: value } : row));
}

// --- save ------------------------------------------------------------------

function parseEquals(): EdgeConditionEquals | undefined {
  if (equalsType.value === 'number') {
    const num = Number(equalsRaw.value);
    return Number.isNaN(num) ? undefined : num;
  }
  if (equalsType.value === 'boolean') {
    return equalsRaw.value === 'true';
  }
  return equalsRaw.value;
}

function build(): PipelineEdge | null {
  const edge: PipelineEdge = {
    source: source.value,
    target: target.value,
    type: edgeType.value,
    mode: mode.value,
  };
  if (edgeType.value === 'conditional') {
    if (!path.value.trim()) {
      formError.value = 'A conditional edge needs a path.';
      return null;
    }
    const equals = parseEquals();
    if (equals === undefined) {
      formError.value = 'A conditional edge needs an equals value.';
      return null;
    }
    edge.path = path.value.trim();
    edge.equals = equals;
  }
  if (mode.value === 'binding') {
    if (bindingBlocked.value) {
      formError.value = 'Binding requires source outputSchema and target inputSchema.';
      return null;
    }
    const validRows = rows.value.filter((row) => row.from.trim() && row.to.trim());
    if (validRows.length === 0) {
      formError.value = 'Binding mode needs at least one from→to row.';
      return null;
    }
    edge.bindings = validRows.map((row) => ({ from: row.from.trim(), to: row.to.trim() }));
  }
  return edge;
}

function save(): void {
  const index = controller.editingEdgeIndex.value;
  if (index === null) return;
  formError.value = null;
  const edge = build();
  if (!edge) return;
  design.replaceEdge(index, edge);
  controller.closeEdgeForm();
}

function cancel(): void {
  controller.closeEdgeForm();
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open" class="form-backdrop" @click.self="cancel">
      <section class="form-panel" role="dialog" aria-label="Edit edge">
        <header class="form-head">
          <h2>Edit edge</h2>
          <code class="form-id">{{ source }} → {{ target }}</code>
        </header>

        <div class="form-body">
          <label class="field">
            <span class="field-label">Type</span>
            <select v-model="edgeType" class="field-input">
              <option value="success">success</option>
              <option value="always">always</option>
              <option value="conditional">conditional</option>
            </select>
          </label>

          <template v-if="edgeType === 'conditional'">
            <label class="field">
              <span class="field-label">Path *</span>
              <input v-model="path" type="text" class="field-input" placeholder="output.field" />
            </label>
            <div class="field-row">
              <label class="field grow">
                <span class="field-label">Equals type</span>
                <select v-model="equalsType" class="field-input">
                  <option value="string">string</option>
                  <option value="number">number</option>
                  <option value="boolean">boolean</option>
                </select>
              </label>
              <label class="field grow">
                <span class="field-label">Equals value *</span>
                <select v-if="equalsType === 'boolean'" v-model="equalsRaw" class="field-input">
                  <option value="true">true</option>
                  <option value="false">false</option>
                </select>
                <input v-else v-model="equalsRaw" type="text" class="field-input" />
              </label>
            </div>
          </template>

          <label class="field">
            <span class="field-label">Data mode</span>
            <select v-model="mode" class="field-input">
              <option value="none">none</option>
              <option value="context">context</option>
              <option value="binding">binding</option>
            </select>
          </label>

          <div v-if="mode === 'binding'" class="binding">
            <p v-if="bindingBlocked" class="field-error">
              Binding needs source <code>outputSchema</code> and target <code>inputSchema</code>.
              <span v-if="!sourceHasOutput">Source is missing outputSchema. </span>
              <span v-if="!targetHasInput">Target is missing inputSchema.</span>
            </p>
            <div class="binding-head">
              <span>Field mapping (from → to)</span>
              <button type="button" class="row-btn" @click="addRow">
                <Plus :size="13" :stroke-width="2" /> Row
              </button>
            </div>
            <div v-for="(row, index) in rows" :key="index" class="binding-row">
              <input
                class="field-input"
                list="from-keys"
                placeholder="source output"
                :value="row.from"
                @input="updateRow(index, 'from', ($event.target as HTMLInputElement).value)"
              />
              <span class="arrow">→</span>
              <input
                class="field-input"
                list="to-keys"
                placeholder="target input"
                :value="row.to"
                @input="updateRow(index, 'to', ($event.target as HTMLInputElement).value)"
              />
              <button type="button" class="row-btn danger" @click="removeRow(index)">
                <Trash2 :size="13" :stroke-width="2" />
              </button>
            </div>
            <datalist id="from-keys">
              <option v-for="key in fromKeys" :key="key" :value="key" />
            </datalist>
            <datalist id="to-keys">
              <option v-for="key in toKeys" :key="key" :value="key" />
            </datalist>
          </div>
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
  width: min(520px, 92%);
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

.field {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  margin-bottom: 0.55rem;
}

.field-row {
  display: flex;
  gap: 0.6rem;
}

.field.grow {
  flex: 1;
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

.binding {
  margin-top: 0.4rem;
  padding: 0.5rem 0.6rem;
  border: 1px dashed var(--border-color);
  border-radius: 8px;
}

.binding-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.4rem;
  font-size: 0.74rem;
  color: var(--text-secondary);
}

.binding-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-bottom: 0.35rem;
}

.arrow {
  color: var(--text-muted);
}

.row-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0.25rem 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.72rem;
  cursor: pointer;
}

.row-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.row-btn.danger:hover {
  color: var(--bg-danger, #d9534f);
}

.field-error {
  margin: 0 0 0.4rem;
  font-size: 0.72rem;
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
