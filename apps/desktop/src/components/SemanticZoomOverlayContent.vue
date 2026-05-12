<script setup lang="ts">
import { computed } from 'vue';
import type { SequentialThinkingViewNode, ToolProvenance, WorkflowStepEntry } from '../lib/pipeline-types';
import { dispatchResource } from '../lib/resource-mime-dispatcher';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import ThinkingTraceView from './ThinkingTraceView.vue';
import PromptAssemblyPreview from './PromptAssemblyPreview.vue';
import ResourcePreview from './ResourcePreview.vue';
import WorkflowStepProgress from './WorkflowStepProgress.vue';

type ThinkingTraceEntry = NonNullable<SequentialThinkingViewNode['thinking_trace']>[number];

const props = defineProps<{
  assembledPrompt: string | null;
  nodeOutput: string | null;
  thinkingTrace: ThinkingTraceEntry[] | null;
  toolProvenance: ToolProvenance | null;
  workflowSteps: WorkflowStepEntry[] | null;
}>();

function textMimeType(text: string): string {
  const trimmed = text.trim();
  return trimmed.startsWith('{') || trimmed.startsWith('[') ? 'application/json' : 'text/markdown';
}

const nodeOutputResource = computed(() => props.nodeOutput
  ? dispatchResource({
    uri: 'pipeline-node-output',
    mimeType: textMimeType(props.nodeOutput),
    text: props.nodeOutput,
  })
  : null);

const toolArgsResource = computed(() => props.toolProvenance?.resolvedArgs
  ? dispatchResource({
    uri: 'pipeline-tool-args',
    mimeType: 'application/json',
    text: JSON.stringify(props.toolProvenance.resolvedArgs),
  })
  : null);

const responseSummaryResource = computed(() => props.toolProvenance?.responseSummary
  ? dispatchResource({
    uri: 'pipeline-tool-response-summary',
    mimeType: 'text/markdown',
    text: props.toolProvenance.responseSummary,
  })
  : null);
</script>

<template>
  <div class="flex flex-col gap-3">
    <PromptAssemblyPreview v-if="props.assembledPrompt" :assembled-prompt="props.assembledPrompt" />

    <ThinkingTraceView v-if="props.thinkingTrace && props.thinkingTrace.length > 0" :trace="props.thinkingTrace" />

    <WorkflowStepProgress v-if="props.workflowSteps && props.workflowSteps.length > 0" :steps="props.workflowSteps" />

    <Accordion
      v-if="props.nodeOutput || props.toolProvenance"
      type="multiple"
      :default-value="['output', 'tool-provenance']"
    >
      <AccordionItem v-if="props.nodeOutput" value="output">
        <AccordionTrigger class="cursor-pointer list-none px-0 py-2 font-display text-caption font-semibold text-text-primary before:content-['▸_'] data-[state=open]:before:content-['▾_']">Output</AccordionTrigger>
        <AccordionContent>
          <ResourcePreview v-if="nodeOutputResource" :dispatched="nodeOutputResource" />
        </AccordionContent>
      </AccordionItem>

      <AccordionItem v-if="props.toolProvenance" value="tool-provenance">
        <AccordionTrigger class="cursor-pointer list-none px-0 py-2 font-display text-caption font-semibold text-text-primary before:content-['▸_'] data-[state=open]:before:content-['▾_']">Tool Provenance</AccordionTrigger>
        <AccordionContent>
          <div class="mt-2 flex items-center justify-between gap-2">
            <span class="font-mono text-tiny font-semibold text-text-primary">Tool</span>
            <strong>{{ props.toolProvenance.toolName }}</strong>
          </div>
          <ResourcePreview v-if="toolArgsResource" class="mt-2" :dispatched="toolArgsResource" />
          <ResourcePreview v-if="responseSummaryResource" class="mt-2" :dispatched="responseSummaryResource" />
        </AccordionContent>
      </AccordionItem>
    </Accordion>

    <div
      v-if="!props.assembledPrompt && !props.thinkingTrace && !props.workflowSteps && !props.nodeOutput && !props.toolProvenance"
      class="p-8 text-center text-caption text-text-secondary"
    >
      This node hasn't produced any output yet.
    </div>
  </div>
</template>
