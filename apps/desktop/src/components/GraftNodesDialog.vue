<script setup lang="ts">
import { computed, toRef, watch } from 'vue';
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
import type { PipelineEdge } from '../lib/pipeline-types-edge';
import type { PipelineNode } from '../lib/pipeline-types-node';
import { useGraftNodes } from '../composables/useGraftNodes';
import GraftNodesDialogTypeFields from './GraftNodesDialogTypeFields.vue';

const props = defineProps<{
  existingNodeIds: string[];
  workflows: string[];
  busy?: boolean;
  sourceNodeId?: string | null;
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

watch(
  () => props.sourceNodeId,
  (sourceNodeId) => {
    connectFrom.value = sourceNodeId?.trim() || '';
  },
  { immediate: true },
);

const connectFromValue = computed({
  get: () => connectFrom.value || NO_EDGE_VALUE,
  set: (value: string) => { connectFrom.value = value === NO_EDGE_VALUE ? '' : value; },
});

const connectToValue = computed({
  get: () => connectTo.value || NO_EDGE_VALUE,
  set: (value: string) => { connectTo.value = value === NO_EDGE_VALUE ? '' : value; },
});
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && emit('cancel')">
    <DialogContent class="gap-0 p-0 sm:max-w-[48rem] w-[min(48rem,100%)] max-h-[min(90vh,52rem)] overflow-auto" :show-close-button="false">
      <DialogHeader class="flex flex-row items-start justify-between gap-4 pt-6 px-6">
        <div class="grid gap-[0.35rem] text-left">
          <DialogTitle>Graft nodes</DialogTitle>
          <DialogDescription>Add one node to the running pipeline and connect it to existing nodes.</DialogDescription>
        </div>
        <DialogClose as-child>
          <Button type="button" variant="ghost">Close</Button>
        </DialogClose>
      </DialogHeader>

      <form class="grid gap-4 px-6 pb-6" @submit.prevent="handleSubmit">
        <div class="grid grid-cols-[repeat(auto-fit,minmax(12rem,1fr))] gap-3">
          <div class="grid min-w-0 gap-[0.35rem] text-[0.75rem] text-text-secondary">
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

          <div class="grid min-w-0 gap-[0.35rem] text-[0.75rem] text-text-secondary">
            <Label for="graft-node-id">Node ID</Label>
            <Input
              id="graft-node-id"
              :model-value="nodeId"
              type="text"
              placeholder="Optional custom ID"
              @update:model-value="(val: string | number) => nodeId = String(val).trim()"
            />
          </div>

          <div class="grid min-w-0 gap-[0.35rem] text-[0.75rem] text-text-secondary">
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

          <div class="grid min-w-0 gap-[0.35rem] text-[0.75rem] text-text-secondary">
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

          <div class="grid min-w-0 gap-[0.35rem] text-[0.75rem] text-text-secondary">
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

        <div class="grid grid-cols-[repeat(auto-fit,minmax(12rem,1fr))] gap-3">
          <GraftNodesDialogTypeFields
            v-model:workflow-id="workflowId"
            v-model:workflow-agent="workflowAgent"
            v-model:workflow-prompt="workflowPrompt"
            v-model:prompt-agent="promptAgent"
            v-model:prompt-template="promptTemplate"
            v-model:thinking-topic="thinkingTopic"
            v-model:thinking-agent="thinkingAgent"
            v-model:thinking-total-thoughts="thinkingTotalThoughts"
            v-model:thinking-analysis-type="thinkingAnalysisType"
            v-model:tool-name="toolName"
            v-model:tool-args="toolArgs"
            v-model:tool-args-valid="toolArgsValid"
            :node-type="nodeType"
            :workflows="workflows"
            :workflow-list-id="workflowListId"
          />
        </div>

        <p class="m-0 text-[0.75rem] text-text-secondary"><strong>Existing node IDs:</strong> {{ existingNodeIds.join(', ') || 'No nodes available.' }}</p>
        <p v-if="formError" class="m-0 text-[0.75rem] text-status-abandoned" role="alert">{{ formError }}</p>

        <DialogFooter class="pt-2">
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
