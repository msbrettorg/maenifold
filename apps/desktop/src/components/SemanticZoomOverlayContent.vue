<script setup lang="ts">
import type { SequentialThinkingViewNode, ToolProvenance, WorkflowStepEntry } from '../lib/pipeline-types';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import ThinkingTraceView from './ThinkingTraceView.vue';
import PromptAssemblyPreview from './PromptAssemblyPreview.vue';
import WorkflowStepProgress from './WorkflowStepProgress.vue';

type ThinkingTraceEntry = NonNullable<SequentialThinkingViewNode['thinking_trace']>[number];

const props = defineProps<{
  assembledPrompt: string | null;
  nodeOutput: string | null;
  thinkingTrace: ThinkingTraceEntry[] | null;
  toolProvenance: ToolProvenance | null;
  workflowSteps: WorkflowStepEntry[] | null;
}>();

function formatJson(value: Record<string, unknown> | null | undefined): string {
  return value ? JSON.stringify(value, null, 2) : '—';
}
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
          <pre class="mt-2 mb-0 mx-0 max-h-[300px] overflow-y-auto rounded-lg bg-bg-app p-3 font-mono text-text-secondary text-tiny leading-[1.5] whitespace-pre-wrap break-words">{{ props.nodeOutput }}</pre>
        </AccordionContent>
      </AccordionItem>

      <AccordionItem v-if="props.toolProvenance" value="tool-provenance">
        <AccordionTrigger class="cursor-pointer list-none px-0 py-2 font-display text-caption font-semibold text-text-primary before:content-['▸_'] data-[state=open]:before:content-['▾_']">Tool Provenance</AccordionTrigger>
        <AccordionContent>
          <div class="mt-2 flex items-center justify-between gap-2">
            <span class="font-mono text-tiny font-semibold text-text-primary">Tool</span>
            <strong>{{ props.toolProvenance.toolName }}</strong>
          </div>
          <pre class="mt-2 mb-0 mx-0 max-h-[300px] overflow-y-auto rounded-lg bg-bg-app p-3 font-mono text-text-secondary text-tiny leading-[1.5] whitespace-pre-wrap break-words">{{ formatJson(props.toolProvenance.resolvedArgs ?? null) }}</pre>
          <pre v-if="props.toolProvenance.responseSummary" class="mt-2 mb-0 mx-0 max-h-[300px] overflow-y-auto rounded-lg bg-bg-app p-3 font-mono text-text-secondary text-tiny leading-[1.5] whitespace-pre-wrap break-words">{{ props.toolProvenance.responseSummary }}</pre>
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
