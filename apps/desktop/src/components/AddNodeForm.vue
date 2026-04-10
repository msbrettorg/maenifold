<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useAddNodeForm } from '../composables/useAddNodeForm';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';
import { groupToolsByCategory } from '../lib/tool-categories';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import SchemaForm from './SchemaForm.vue';

interface SubmitNode {
  (type: 'workflow', fields: PipelineNodeDraftByType['workflow']): void;
  (type: 'prompt', fields: PipelineNodeDraftByType['prompt']): void;
  (type: 'sequentialThinking', fields: PipelineNodeDraftByType['sequentialThinking']): void;
  (type: 'tool', fields: PipelineNodeDraftByType['tool']): void;
}

const props = defineProps<{
  workflows: string[];
  hasSelection: boolean;
  submitNode: SubmitNode;
}>();

const emit = defineEmits<{
  (e: 'deleteSelected'): void;
  (e: 'autoLayout', direction: 'LR' | 'TB'): void;
}>();

const {
  workflowListId,
  panelOpen,
  nodeType,
  workflowId,
  workflowAgent,
  workflowPrompt,
  promptAgent,
  promptTemplate,
  thinkingTopic,
  thinkingAgent,
  thinkingTotalThoughts,
  thinkingAnalysisType,
  toolName,
  toolArgs,
  toolArgsValid,
  canAdd,
  handleSubmit,
} = useAddNodeForm({ submitNode: props.submitNode });

const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();
const pipelineStore = usePipelineStore();
const toolOptions = computed(() => groupToolsByCategory(pipelineStore.tools));
const selectedToolSchema = computed(
  () => pipelineStore.tools.find((tool) => tool.name === toolName.value)?.inputSchema ?? null,
);
const attemptedToolLoad = ref(false);
const loadingToolSchemas = ref(false);

function getProbe(agentName: string) {
  return capabilitiesStore.getProbe(agentName);
}

function formatAgentOptionLabel(agentName: string): string {
  const probe = getProbe(agentName);
  if (!probe) return agentName;
  if (probe.error) return `${agentName} ⚠`;
  if (probe.modes.length <= 2 && probe.modes.length > 0) return `${agentName} — ${probe.modes.join(', ')}`;
  if (probe.modes.length > 2) return `${agentName} (${probe.modes.length} modes)`;
  return probe.models.length ? `${agentName} (${probe.models.length} model${probe.models.length === 1 ? '' : 's'})` : agentName;
}

watch(
  () => [nodeType.value === 'tool', pipelineStore.tools.length] as const,
  async ([toolNodeSelected, toolCount]) => {
    if (!toolNodeSelected || toolCount > 0 || attemptedToolLoad.value) return;
    attemptedToolLoad.value = true;
    loadingToolSchemas.value = true;
    try {
      await pipelineStore.refreshTools();
    } finally {
      loadingToolSchemas.value = false;
    }
  },
  { immediate: true },
);

function onToolArgsChange(data: Record<string, unknown>): void {
  toolArgs.value = data;
}

function onToolArgsValidation(state: { valid: boolean }): void {
  toolArgsValid.value = state.valid;
}
</script>

<template>
  <form class="composer-toolbar" @submit.prevent="handleSubmit">
    <div class="toolbar-row toolbar-row-top">
      <select v-model="nodeType" class="field-select" aria-label="Node type">
        <option value="workflow">Workflow</option>
        <option value="prompt">Prompt</option>
        <option value="sequentialThinking">Sequential Thinking</option>
        <option value="tool">Tool</option>
      </select>
      <button
        type="button"
        class="btn btn-ghost"
        :aria-expanded="panelOpen"
        @click="panelOpen = !panelOpen"
      >
        {{ panelOpen ? 'Hide Fields' : 'Show Fields' }}
      </button>
      <button type="submit" class="btn btn-primary" :disabled="!canAdd">Add Node</button>
      <button
        v-if="hasSelection"
        type="button"
        class="btn btn-outline-danger"
        @click="emit('deleteSelected')"
      >
        Delete Selected
      </button>
      <button type="button" class="btn btn-ghost" @click="emit('autoLayout', 'LR')">↔ Layout LR</button>
      <button type="button" class="btn btn-ghost" @click="emit('autoLayout', 'TB')">↕ Layout TB</button>
    </div>

    <div v-show="panelOpen" class="toolbar-row toolbar-row-fields">
      <template v-if="nodeType === 'workflow'">
        <label>
          <span>Workflow ID</span>
          <input
            v-model.trim="workflowId"
            class="field-input"
            :list="workflowListId"
            type="text"
            placeholder="Workflow ID"
          />
        </label>
        <datalist :id="workflowListId">
          <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
        </datalist>
        <label>
          <span>Agent</span>
          <select v-model="workflowAgent" class="field-input"><option value="">Select agent…</option><option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option></select>
        </label>
        <label class="field-wide">
          <span>Prompt</span>
          <textarea
            v-model.trim="workflowPrompt"
            class="field-textarea"
            rows="3"
            placeholder="Optional workflow prompt"
          />
        </label>
      </template>

      <template v-else-if="nodeType === 'prompt'">
        <label>
          <span>Agent</span>
          <select v-model="promptAgent" class="field-input"><option value="" disabled>Select agent…</option><option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option></select>
        </label>
        <label class="field-wide">
          <span>Prompt</span>
          <textarea
            v-model.trim="promptTemplate"
            class="field-textarea"
            rows="3"
            placeholder="Required prompt template"
          />
        </label>
      </template>

      <template v-else-if="nodeType === 'sequentialThinking'">
        <label>
          <span>Topic</span>
          <input v-model.trim="thinkingTopic" class="field-input" type="text" placeholder="Topic" />
        </label>
        <label>
          <span>Agent</span>
          <select v-model="thinkingAgent" class="field-input"><option value="" disabled>Select agent…</option><option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option></select>
        </label>
        <label>
          <span>Total thoughts</span>
          <input
            v-model="thinkingTotalThoughts"
            class="field-input"
            type="number"
            min="1"
            inputmode="numeric"
            placeholder="5"
          />
        </label>
        <label>
          <span>Analysis type</span>
          <input
            v-model.trim="thinkingAnalysisType"
            class="field-input"
            type="text"
            placeholder="Optional analysis type"
          />
        </label>
      </template>

      <template v-else>
        <label>
          <span>Tool Name</span>
          <select v-model="toolName" class="field-input">
            <option value="" disabled>Select tool…</option>
            <optgroup v-for="cat in toolOptions" :key="cat.group" :label="cat.group">
              <option v-for="tool in cat.tools" :key="tool.name" :value="tool.name">{{ tool.name }}</option>
            </optgroup>
          </select>
          <small v-if="loadingToolSchemas">Loading tool schema…</small>
        </label>
        <div class="field-wide field-block">
          <span>Tool Args</span>
          <SchemaForm
            :schema="selectedToolSchema"
            :data="toolArgs"
            @change="onToolArgsChange"
            @validation="onToolArgsValidation"
          />
        </div>
      </template>
    </div>
  </form>
</template>

<style scoped>
.composer-toolbar { position: absolute; top: 0.75rem; left: 0.75rem; z-index: 5; display: grid; gap: 0.6rem; padding: 0.75rem; min-width: min(44rem, calc(100% - 1.5rem)); border: 1px solid var(--border-color); border-radius: 0.75rem; background: color-mix(in srgb, var(--bg-main) 94%, transparent); box-shadow: 0 6px 24px color-mix(in srgb, var(--bg-main) 12%, transparent); }
.toolbar-row { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: flex-start; }
.toolbar-row-fields { display: grid; grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr)); }
label { display: grid; gap: 0.35rem; min-width: 0; color: var(--text-secondary); font-size: var(--text-sm); }
.field-block { display: grid; gap: 0.35rem; min-width: 0; color: var(--text-secondary); font-size: var(--text-sm); }
.field-wide { grid-column: 1 / -1; }
.toggle-field { align-content: end; }
.toggle-button { min-width: 7.5rem; }
</style>
