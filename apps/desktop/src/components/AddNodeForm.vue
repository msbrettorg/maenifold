<script setup lang="ts">
import { computed } from 'vue';
import { useAddNodeForm } from '../composables/useAddNodeForm';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import AddNodeFormFields from './AddNodeFormFields.vue';
import AddNodeFormTypeSelector from './AddNodeFormTypeSelector.vue';
import { ScrollArea } from '@/components/ui/scroll-area';

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
  <form
    class="h-full p-4 bg-surface"
    @submit.prevent="handleSubmit"
  >
    <ScrollArea class="h-full">
    <div class="grid gap-4 content-start">
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
    </div>
    </ScrollArea>
  </form>
</template>
