<script setup lang="ts">
import { computed } from 'vue';
import { useAddNodeForm } from '../composables/useAddNodeForm';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import AddNodeFormFields from './AddNodeFormFields.vue';
import AddNodeFormTypeSelector from './AddNodeFormTypeSelector.vue';

interface SubmitNode {
  (type: 'workflow', fields: PipelineNodeDraftByType['workflow']): void;
  (type: 'prompt', fields: PipelineNodeDraftByType['prompt']): void;
  (type: 'sequentialThinking', fields: PipelineNodeDraftByType['sequentialThinking']): void;
  (type: 'tool', fields: PipelineNodeDraftByType['tool']): void;
}

const props = withDefaults(defineProps<{
  workflows: string[];
  submitNode: SubmitNode;
  hasSelection?: boolean;
}>(), {
  hasSelection: false,
});

const {
  workflowListId,
  nodeType,
  workflowId,
  workflowAgent,
  workflowAgentConfig,
  workflowPrompt,
  promptAgent,
  promptAgentConfig,
  promptTemplate,
  thinkingTopic,
  thinkingAgent,
  thinkingAgentConfig,
  thinkingTotalThoughts,
  thinkingAnalysisType,
  toolName,
  toolAgent,
  toolAgentConfig,
  toolArgs,
  toolArgsValid,
  canAdd,
  cwd,
  handleSubmit,
} = useAddNodeForm({ submitNode: props.submitNode });

const addButtonLabel = computed(() => (nodeType.value === 'tool' ? 'Add Tool' : 'Add Node'));
</script>

<template>
  <form class="composer-toolbar" @submit.prevent="handleSubmit">
    <AddNodeFormTypeSelector
      v-model="nodeType"
      :can-add="canAdd"
      :submit-label="addButtonLabel"
    />
    <AddNodeFormFields
      :workflows="workflows"
      :workflow-list-id="workflowListId"
      :node-type="nodeType"
      :workflow-id="workflowId"
      :workflow-agent="workflowAgent"
      :workflow-agent-config="workflowAgentConfig"
      :workflow-prompt="workflowPrompt"
      :prompt-agent="promptAgent"
      :prompt-agent-config="promptAgentConfig"
      :prompt-template="promptTemplate"
      :thinking-topic="thinkingTopic"
      :thinking-agent="thinkingAgent"
      :thinking-agent-config="thinkingAgentConfig"
      :thinking-total-thoughts="thinkingTotalThoughts"
      :thinking-analysis-type="thinkingAnalysisType"
      :tool-name="toolName"
      :tool-agent="toolAgent"
      :tool-agent-config="toolAgentConfig"
      :tool-args="toolArgs"
      :tool-args-valid="toolArgsValid"
      :can-submit="canAdd"
      :cwd="cwd"
      @update:workflow-id="workflowId = $event"
      @update:workflow-agent="workflowAgent = $event"
      @update:workflow-agent-config="workflowAgentConfig = $event"
      @update:workflow-prompt="workflowPrompt = $event"
      @update:prompt-agent="promptAgent = $event"
      @update:prompt-agent-config="promptAgentConfig = $event"
      @update:prompt-template="promptTemplate = $event"
      @update:thinking-topic="thinkingTopic = $event"
      @update:thinking-agent="thinkingAgent = $event"
      @update:thinking-agent-config="thinkingAgentConfig = $event"
      @update:thinking-total-thoughts="thinkingTotalThoughts = $event"
      @update:thinking-analysis-type="thinkingAnalysisType = $event"
      @update:tool-name="toolName = $event"
      @update:tool-agent="toolAgent = $event"
      @update:tool-agent-config="toolAgentConfig = $event"
      @update:tool-args="toolArgs = $event"
      @update:cwd="cwd = $event ?? ''"
      @validation="toolArgsValid = $event.valid"
      @submit-request="handleSubmit"
    />
  </form>
</template>

<style scoped>
.composer-toolbar {
  display: grid;
  gap: 1rem;
  align-content: start;
  height: 100%;
  padding: 1rem;
  background: var(--surface);
  overflow-y: auto;
}

.composer-toolbar :deep(.toolbar-row) {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
}

.composer-toolbar :deep(.toolbar-type-field) {
  min-width: 12rem;
}

.composer-toolbar :deep(.toolbar-actions) {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.composer-toolbar :deep(.toolbar-section) {
  display: grid;
  gap: 0.8rem;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
}

.composer-toolbar :deep(label) {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-dim);
  font-size: 0.75rem;
}

.composer-toolbar :deep(.field-wide) {
  grid-column: 1 / -1;
}

.composer-toolbar :deep(.agent-capabilities),
.composer-toolbar :deep(.agent-mode-badges) {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.composer-toolbar :deep(.agent-capability),
.composer-toolbar :deep(.mode-badge) {
  display: inline-flex;
  align-items: center;
  padding: 0.15rem 0.45rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: 4px;
  font-size: var(--text-xs);
}

.composer-toolbar :deep(.agent-config-panel) {
  display: grid;
  gap: 0.65rem;
  padding: 0.75rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--surface);
}

.composer-toolbar :deep(.agent-config-grid) {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
}

.composer-toolbar :deep(.mode-badge) {
  background: var(--color-bg-app);
  color: var(--color-text-secondary);
  cursor: pointer;
}

.composer-toolbar :deep(.mode-badge.selected) {
  border-color: var(--color-brand);
  color: var(--color-text-primary);
  background: color-mix(in srgb, var(--color-brand) 45%, transparent);
}
</style>
