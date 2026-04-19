<script setup lang="ts">
import { computed, toRef } from 'vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import type { PipelineEdge, PipelineNode } from '../lib/pipeline-types';
import { useGraftNodes } from '../composables/useGraftNodes';
import ToolPaletteFields from './ToolPaletteFields.vue';

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

const NO_EDGE_VALUE = '__none__';

const connectFromValue = computed({
  get: () => connectFrom.value || NO_EDGE_VALUE,
  set: (value: string) => {
    connectFrom.value = value === NO_EDGE_VALUE ? '' : value;
  },
});

const connectToValue = computed({
  get: () => connectTo.value || NO_EDGE_VALUE,
  set: (value: string) => {
    connectTo.value = value === NO_EDGE_VALUE ? '' : value;
  },
});
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && emit('cancel')">
    <DialogContent class="graft-dialog gap-0 p-0 sm:max-w-[48rem]" :show-close-button="false">
      <DialogHeader class="dialog-header">
        <div class="dialog-heading">
          <DialogTitle>Graft nodes</DialogTitle>
          <DialogDescription>Add one node to the running pipeline and connect it to existing nodes.</DialogDescription>
        </div>
        <DialogClose as-child>
          <Button type="button" variant="ghost">Close</Button>
        </DialogClose>
      </DialogHeader>

      <form class="dialog-form" @submit.prevent="handleSubmit">
        <div class="form-grid">
          <div class="form-field">
            <Label for="graft-node-type">Node type</Label>
            <Select v-model="nodeType">
              <SelectTrigger id="graft-node-type" class="w-full">
                <SelectValue placeholder="Select node type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="workflow">Workflow</SelectItem>
                <SelectItem value="prompt">Prompt</SelectItem>
                <SelectItem value="sequentialThinking">Sequential Thinking</SelectItem>
                <SelectItem value="tool">Tool</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="form-field">
            <Label for="graft-node-id">Node ID</Label>
            <Input id="graft-node-id" v-model.trim="nodeId" type="text" placeholder="Optional custom ID" />
          </div>

          <div class="form-field">
            <Label for="graft-connect-from">Run after</Label>
            <Select v-model="connectFromValue">
              <SelectTrigger id="graft-connect-from" class="w-full">
                <SelectValue placeholder="No upstream edge" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NO_EDGE_VALUE">No upstream edge</SelectItem>
                <SelectItem v-for="existingNodeId in existingNodeIds" :key="`from-${existingNodeId}`" :value="existingNodeId">
                  {{ existingNodeId }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="form-field">
            <Label for="graft-connect-to">Continue to</Label>
            <Select v-model="connectToValue">
              <SelectTrigger id="graft-connect-to" class="w-full">
                <SelectValue placeholder="No downstream edge" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NO_EDGE_VALUE">No downstream edge</SelectItem>
                <SelectItem v-for="existingNodeId in existingNodeIds" :key="`to-${existingNodeId}`" :value="existingNodeId">
                  {{ existingNodeId }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="form-field">
            <Label for="graft-edge-type">Edge type</Label>
            <Select v-model="edgeType">
              <SelectTrigger id="graft-edge-type" class="w-full">
                <SelectValue placeholder="Select edge type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="success">Success</SelectItem>
                <SelectItem value="always">Always</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div class="form-grid">
          <template v-if="nodeType === 'workflow'">
            <div class="form-field">
              <Label for="graft-workflow-id">Workflow ID</Label>
              <Input id="graft-workflow-id" v-model.trim="workflowId" :list="workflowListId" type="text" placeholder="Workflow ID" />
            </div>
            <datalist :id="workflowListId">
              <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
            </datalist>
            <div class="form-field">
              <Label for="graft-workflow-agent">Agent</Label>
              <Input id="graft-workflow-agent" v-model.trim="workflowAgent" type="text" placeholder="Optional workflow agent" />
            </div>
            <div class="form-field field-wide">
              <Label for="graft-workflow-prompt">Prompt</Label>
              <Textarea id="graft-workflow-prompt" v-model.trim="workflowPrompt" class="graft-textarea" rows="3" placeholder="Optional workflow prompt" />
            </div>
          </template>

          <template v-else-if="nodeType === 'prompt'">
            <div class="form-field">
              <Label for="graft-prompt-agent">Agent</Label>
              <Input id="graft-prompt-agent" v-model.trim="promptAgent" type="text" placeholder="Agent name" />
            </div>
            <div class="form-field field-wide">
              <Label for="graft-prompt-template">Prompt</Label>
              <Textarea id="graft-prompt-template" v-model.trim="promptTemplate" class="graft-textarea" rows="4" placeholder="Required prompt template" />
            </div>
          </template>

          <template v-else-if="nodeType === 'sequentialThinking'">
            <div class="form-field">
              <Label for="graft-thinking-topic">Topic</Label>
              <Input id="graft-thinking-topic" v-model.trim="thinkingTopic" type="text" placeholder="Topic" />
            </div>
            <div class="form-field">
              <Label for="graft-thinking-agent">Agent</Label>
              <Input id="graft-thinking-agent" v-model.trim="thinkingAgent" type="text" placeholder="Agent name" />
            </div>
            <div class="form-field">
              <Label for="graft-thinking-total-thoughts">Total thoughts</Label>
              <Input id="graft-thinking-total-thoughts" v-model="thinkingTotalThoughts" type="number" min="1" inputmode="numeric" placeholder="5" />
            </div>
            <div class="form-field">
              <Label for="graft-thinking-analysis-type">Analysis type</Label>
              <Input id="graft-thinking-analysis-type" v-model.trim="thinkingAnalysisType" type="text" placeholder="Optional analysis type" />
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
        </div>

        <p class="existing-nodes"><strong>Existing node IDs:</strong> {{ existingNodeIds.join(', ') || 'No nodes available.' }}</p>
        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

        <DialogFooter class="dialog-actions">
          <DialogClose as-child>
            <Button type="button" variant="ghost" :disabled="busy">Cancel</Button>
          </DialogClose>
          <Button type="submit" :disabled="!canSubmit || busy">
            {{ busy ? 'Grafting…' : 'Graft Node' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.graft-dialog {
  width: min(48rem, 100%);
  max-height: min(90vh, 52rem);
  overflow: auto;
}

.dialog-heading {
  display: grid;
  gap: 0.35rem;
  text-align: left;
}

.dialog-header {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.5rem 1.5rem 0;
}

.dialog-form {
  display: grid;
  gap: 1rem;
  padding: 0 1.5rem 1.5rem;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: 0.75rem;
}

.form-field {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-dim);
  font-size: 0.75rem;
}

.field-wide {
  grid-column: 1 / -1;
}

.field-block {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-dim);
  font-size: 0.75rem;
}

.graft-textarea {
  resize: vertical;
  min-height: 5rem;
}

.existing-nodes,
.form-error {
  margin: 0;
  font-size: 0.75rem;
}

.existing-nodes {
  color: var(--text-dim);
}

.form-error {
  color: var(--color-status-abandoned);
}

.dialog-actions {
  padding-top: 0.5rem;
}
</style>
