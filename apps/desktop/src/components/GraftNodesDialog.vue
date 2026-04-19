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
          <label>
            <span>Node type</span>
            <select
              v-model="nodeType"
              class="w-full bg-[var(--surface)] text-foreground border border-[rgba(66,71,83,0.15)] rounded-[4px] px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
            >
              <option value="workflow">Workflow</option>
              <option value="prompt">Prompt</option>
              <option value="sequentialThinking">Sequential Thinking</option>
              <option value="tool">Tool</option>
            </select>
          </label>

          <label>
            <span>Node ID</span>
            <Input v-model.trim="nodeId" type="text" placeholder="Optional custom ID" />
          </label>

          <label>
            <span>Run after</span>
            <select
              v-model="connectFrom"
              class="w-full bg-[var(--surface)] text-foreground border border-[rgba(66,71,83,0.15)] rounded-[4px] px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
            >
              <option value="">No upstream edge</option>
              <option v-for="existingNodeId in existingNodeIds" :key="`from-${existingNodeId}`" :value="existingNodeId">
                {{ existingNodeId }}
              </option>
            </select>
          </label>

          <label>
            <span>Continue to</span>
            <select
              v-model="connectTo"
              class="w-full bg-[var(--surface)] text-foreground border border-[rgba(66,71,83,0.15)] rounded-[4px] px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
            >
              <option value="">No downstream edge</option>
              <option v-for="existingNodeId in existingNodeIds" :key="`to-${existingNodeId}`" :value="existingNodeId">
                {{ existingNodeId }}
              </option>
            </select>
          </label>

          <label>
            <span>Edge type</span>
            <select
              v-model="edgeType"
              class="w-full bg-[var(--surface)] text-foreground border border-[rgba(66,71,83,0.15)] rounded-[4px] px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
            >
              <option value="success">Success</option>
              <option value="always">Always</option>
            </select>
          </label>
        </div>

        <div class="form-grid">
          <template v-if="nodeType === 'workflow'">
            <label>
              <span>Workflow ID</span>
              <Input v-model.trim="workflowId" :list="workflowListId" type="text" placeholder="Workflow ID" />
            </label>
            <datalist :id="workflowListId">
              <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
            </datalist>
            <label>
              <span>Agent</span>
              <Input v-model.trim="workflowAgent" type="text" placeholder="Optional workflow agent" />
            </label>
            <label class="field-wide">
              <span>Prompt</span>
              <Textarea v-model.trim="workflowPrompt" class="graft-textarea" rows="3" placeholder="Optional workflow prompt" />
            </label>
          </template>

          <template v-else-if="nodeType === 'prompt'">
            <label>
              <span>Agent</span>
              <Input v-model.trim="promptAgent" type="text" placeholder="Agent name" />
            </label>
            <label class="field-wide">
              <span>Prompt</span>
              <Textarea v-model.trim="promptTemplate" class="graft-textarea" rows="4" placeholder="Required prompt template" />
            </label>
          </template>

          <template v-else-if="nodeType === 'sequentialThinking'">
            <label>
              <span>Topic</span>
              <Input v-model.trim="thinkingTopic" type="text" placeholder="Topic" />
            </label>
            <label>
              <span>Agent</span>
              <Input v-model.trim="thinkingAgent" type="text" placeholder="Agent name" />
            </label>
            <label>
              <span>Total thoughts</span>
              <Input v-model="thinkingTotalThoughts" type="number" min="1" inputmode="numeric" placeholder="5" />
            </label>
            <label>
              <span>Analysis type</span>
              <Input v-model.trim="thinkingAnalysisType" type="text" placeholder="Optional analysis type" />
            </label>
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

label {
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
