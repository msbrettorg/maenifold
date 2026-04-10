<script setup lang="ts">
import { computed, toRef } from 'vue';
import type { PipelineEdge, PipelineNode } from '../lib/pipeline-types';
import { useGraftNodes } from '../composables/useGraftNodes';
import ToolNodeFields from './ToolNodeFields.vue';

const props = defineProps<{
  existingNodeIds: string[];
  workflows: string[];
  busy?: boolean;
}>();

const emit = defineEmits<{
  (e: 'cancel'): void;
  (e: 'submit', nodes: PipelineNode[], edges: PipelineEdge[]): void;
}>();

const {
  workflowListId,
  nodeType,
  nodeId,
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
  connectFrom,
  connectTo,
  edgeType,
  formError,
  canSubmit,
  handleSubmit,
} = useGraftNodes({
  existingNodeIds: toRef(props, 'existingNodeIds'),
  busy: computed(() => props.busy ?? false),
  onSubmit: (nodes, edges) => emit('submit', nodes, edges),
});
</script>

<template>
  <div class="graft-overlay" @click.self="emit('cancel')">
    <div class="graft-dialog panel" role="dialog" aria-modal="true" aria-labelledby="graft-dialog-title">
      <div class="dialog-header">
        <div>
          <h3 id="graft-dialog-title">Graft Nodes</h3>
          <p>Add one node to the running pipeline and connect it to existing nodes.</p>
        </div>
        <button type="button" class="btn btn-ghost" @click="emit('cancel')">Close</button>
      </div>

      <form class="dialog-form" @submit.prevent="handleSubmit">
        <div class="form-grid">
          <label>
            <span>Node Type</span>
            <select v-model="nodeType" class="field-select">
              <option value="workflow">Workflow</option>
              <option value="prompt">Prompt</option>
              <option value="sequentialThinking">Sequential Thinking</option>
              <option value="tool">Tool</option>
            </select>
          </label>

          <label>
            <span>Node ID</span>
            <input v-model.trim="nodeId" class="field-input" type="text" placeholder="Optional custom ID" />
          </label>

          <label>
            <span>Run After</span>
            <select v-model="connectFrom" class="field-select">
              <option value="">No upstream edge</option>
              <option v-for="existingNodeId in existingNodeIds" :key="`from-${existingNodeId}`" :value="existingNodeId">
                {{ existingNodeId }}
              </option>
            </select>
          </label>

          <label>
            <span>Continue To</span>
            <select v-model="connectTo" class="field-select">
              <option value="">No downstream edge</option>
              <option v-for="existingNodeId in existingNodeIds" :key="`to-${existingNodeId}`" :value="existingNodeId">
                {{ existingNodeId }}
              </option>
            </select>
          </label>

          <label>
            <span>Edge Type</span>
            <select v-model="edgeType" class="field-select">
              <option value="success">Success</option>
              <option value="always">Always</option>
            </select>
          </label>
        </div>

        <div class="form-grid">
          <template v-if="nodeType === 'workflow'">
            <label>
              <span>Workflow ID</span>
              <input v-model.trim="workflowId" class="field-input" :list="workflowListId" type="text" placeholder="Workflow ID" />
            </label>
            <datalist :id="workflowListId">
              <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
            </datalist>
            <label>
              <span>Agent</span>
              <input v-model.trim="workflowAgent" class="field-input" type="text" placeholder="Optional workflow agent" />
            </label>
            <label class="field-wide">
              <span>Prompt</span>
              <textarea v-model.trim="workflowPrompt" class="field-input graft-textarea" rows="3" placeholder="Optional workflow prompt" />
            </label>
          </template>

          <template v-else-if="nodeType === 'prompt'">
            <label>
              <span>Agent</span>
              <input v-model.trim="promptAgent" class="field-input" type="text" placeholder="Agent name" />
            </label>
            <label class="field-wide">
              <span>Prompt</span>
              <textarea v-model.trim="promptTemplate" class="field-input graft-textarea" rows="4" placeholder="Required prompt template" />
            </label>
          </template>

          <template v-else-if="nodeType === 'sequentialThinking'">
            <label>
              <span>Topic</span>
              <input v-model.trim="thinkingTopic" class="field-input" type="text" placeholder="Topic" />
            </label>
            <label>
              <span>Agent</span>
              <input v-model.trim="thinkingAgent" class="field-input" type="text" placeholder="Agent name" />
            </label>
            <label>
              <span>Total thoughts</span>
              <input v-model="thinkingTotalThoughts" class="field-input" type="number" min="1" inputmode="numeric" placeholder="5" />
            </label>
            <label>
              <span>Analysis type</span>
              <input v-model.trim="thinkingAnalysisType" class="field-input" type="text" placeholder="Optional analysis type" />
            </label>
          </template>

          <template v-else>
            <ToolNodeFields
              :active="nodeType === 'tool'"
              :tool-name="toolName"
              :tool-args="toolArgs"
              @update:tool-name="toolName = $event"
              @update:tool-args="toolArgs = $event"
              @validation="toolArgsValid = $event.valid"
            />
          </template>
        </div>

        <p class="existing-nodes"><strong>Existing node IDs:</strong> {{ existingNodeIds.join(', ') || 'No nodes available.' }}</p>
        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

        <div class="dialog-actions">
          <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('cancel')">Cancel</button>
          <button type="submit" class="btn btn-primary" :disabled="!canSubmit || busy">
            {{ busy ? 'Grafting…' : 'Graft Node' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.graft-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: var(--bg-overlay);
}

.graft-dialog {
  width: min(48rem, 100%);
  max-height: min(90vh, 52rem);
  overflow: auto;
}

.dialog-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1rem;
}

.dialog-header h3,
.dialog-header p {
  margin: 0;
}

.dialog-header p {
  margin-top: 0.35rem;
  color: var(--text-secondary);
}

.dialog-form {
  display: grid;
  gap: 1rem;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: 0.75rem;
}

label {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.field-wide {
  grid-column: 1 / -1;
}

.field-block {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.graft-textarea {
  resize: vertical;
  min-height: 5rem;
}

.existing-nodes,
.form-error {
  margin: 0;
  font-size: var(--text-sm);
}

.existing-nodes {
  color: var(--text-secondary);
}

.form-error {
  color: var(--color-status-abandoned);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
