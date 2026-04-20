<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import ToolPaletteFields from './ToolPaletteFields.vue';

defineProps<{
  nodeType: 'workflow' | 'prompt' | 'sequentialThinking' | 'tool';
  workflows: string[];
  workflowListId: string;
}>();

const workflowId = defineModel<string>('workflowId', { required: true });
const workflowAgent = defineModel<string>('workflowAgent', { required: true });
const workflowPrompt = defineModel<string>('workflowPrompt', { required: true });
const promptAgent = defineModel<string>('promptAgent', { required: true });
const promptTemplate = defineModel<string>('promptTemplate', { required: true });
const thinkingTopic = defineModel<string>('thinkingTopic', { required: true });
const thinkingAgent = defineModel<string>('thinkingAgent', { required: true });
const thinkingTotalThoughts = defineModel<number | string>('thinkingTotalThoughts', { required: true });
const thinkingAnalysisType = defineModel<string>('thinkingAnalysisType', { required: true });
const toolName = defineModel<string>('toolName', { required: true });
const toolArgs = defineModel<Record<string, unknown>>('toolArgs', { required: true });
const toolArgsValid = defineModel<boolean>('toolArgsValid', { required: true });
</script>

<template>
  <template v-if="nodeType === 'workflow'">
    <div class="form-field">
      <Label for="graft-workflow-id">Workflow ID</Label>
      <Input
        id="graft-workflow-id"
        :model-value="workflowId"
        :list="workflowListId"
        type="text"
        placeholder="Workflow ID"
        @update:model-value="(val: string | number) => workflowId = String(val).trim()"
      />
    </div>
    <datalist :id="workflowListId">
      <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
    </datalist>
    <div class="form-field">
      <Label for="graft-workflow-agent">Agent</Label>
      <Input
        id="graft-workflow-agent"
        :model-value="workflowAgent"
        type="text"
        placeholder="Optional workflow agent"
        @update:model-value="(val: string | number) => workflowAgent = String(val).trim()"
      />
    </div>
    <div class="form-field field-wide">
      <Label for="graft-workflow-prompt">Prompt</Label>
      <Textarea
        id="graft-workflow-prompt"
        :model-value="workflowPrompt"
        class="graft-textarea"
        rows="3"
        placeholder="Optional workflow prompt"
        @update:model-value="(val: string | number) => workflowPrompt = String(val).trim()"
      />
    </div>
  </template>

  <template v-else-if="nodeType === 'prompt'">
    <div class="form-field">
      <Label for="graft-prompt-agent">Agent</Label>
      <Input
        id="graft-prompt-agent"
        :model-value="promptAgent"
        type="text"
        placeholder="Agent name"
        @update:model-value="(val: string | number) => promptAgent = String(val).trim()"
      />
    </div>
    <div class="form-field field-wide">
      <Label for="graft-prompt-template">Prompt</Label>
      <Textarea
        id="graft-prompt-template"
        :model-value="promptTemplate"
        class="graft-textarea"
        rows="4"
        placeholder="Required prompt template"
        @update:model-value="(val: string | number) => promptTemplate = String(val).trim()"
      />
    </div>
  </template>

  <template v-else-if="nodeType === 'sequentialThinking'">
    <div class="form-field">
      <Label for="graft-thinking-topic">Topic</Label>
      <Input
        id="graft-thinking-topic"
        :model-value="thinkingTopic"
        type="text"
        placeholder="Topic"
        @update:model-value="(val: string | number) => thinkingTopic = String(val).trim()"
      />
    </div>
    <div class="form-field">
      <Label for="graft-thinking-agent">Agent</Label>
      <Input
        id="graft-thinking-agent"
        :model-value="thinkingAgent"
        type="text"
        placeholder="Agent name"
        @update:model-value="(val: string | number) => thinkingAgent = String(val).trim()"
      />
    </div>
    <div class="form-field">
      <Label for="graft-thinking-total-thoughts">Total thoughts</Label>
      <Input
        id="graft-thinking-total-thoughts"
        v-model="thinkingTotalThoughts"
        type="number"
        min="1"
        inputmode="numeric"
        placeholder="5"
      />
    </div>
    <div class="form-field">
      <Label for="graft-thinking-analysis-type">Analysis type</Label>
      <Input
        id="graft-thinking-analysis-type"
        :model-value="thinkingAnalysisType"
        type="text"
        placeholder="Optional analysis type"
        @update:model-value="(val: string | number) => thinkingAnalysisType = String(val).trim()"
      />
    </div>
  </template>

  <template v-else>
    <ToolPaletteFields
      :active="nodeType === 'tool'"
      :tool-name="toolName"
      :tool-args="toolArgs"
      @update:tool-name="toolName = $event"
      @update:tool-args="toolArgs = $event"
      @validation="toolArgsValid = $event.valid"
    />
  </template>
</template>

<style scoped>
.form-field {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-secondary);
  font-size: 0.75rem;
}

.field-wide {
  grid-column: 1 / -1;
}

.graft-textarea {
  resize: vertical;
  min-height: 5rem;
}
</style>
