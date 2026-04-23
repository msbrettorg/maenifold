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
  <div class="flex flex-col gap-[var(--space-3)]">
    <PromptAssemblyPreview v-if="props.assembledPrompt" :assembled-prompt="props.assembledPrompt" />

    <ThinkingTraceView v-if="props.thinkingTrace && props.thinkingTrace.length > 0" :trace="props.thinkingTrace" />

    <WorkflowStepProgress v-if="props.workflowSteps && props.workflowSteps.length > 0" :steps="props.workflowSteps" />

    <Accordion
      v-if="props.nodeOutput || props.toolProvenance"
      type="multiple"
      :default-value="['output', 'tool-provenance']"
    >
      <AccordionItem v-if="props.nodeOutput" value="output">
        <AccordionTrigger class="font-[family-name:var(--font-display)] text-[length:var(--text-caption)] font-[number:var(--weight-title)] text-text-primary cursor-pointer py-[var(--space-2)] px-0 list-none before:content-['▸_'] data-[state=open]:before:content-['▾_']">Output</AccordionTrigger>
        <AccordionContent>
          <pre class="mt-[var(--space-2)] mb-0 mx-0 p-[var(--space-3)] font-mono text-[length:var(--text-tiny)] leading-[1.5] text-text-secondary bg-[var(--color-bg-app)] rounded-[var(--radius-md)] whitespace-pre-wrap break-words max-h-[300px] overflow-y-auto">{{ props.nodeOutput }}</pre>
        </AccordionContent>
      </AccordionItem>

      <AccordionItem v-if="props.toolProvenance" value="tool-provenance">
        <AccordionTrigger class="font-[family-name:var(--font-display)] text-[length:var(--text-caption)] font-[number:var(--weight-title)] text-text-primary cursor-pointer py-[var(--space-2)] px-0 list-none before:content-['▸_'] data-[state=open]:before:content-['▾_']">Tool Provenance</AccordionTrigger>
        <AccordionContent>
          <div class="flex items-center justify-between gap-[var(--space-2)] mt-[var(--space-2)]">
            <span class="font-mono text-[length:var(--text-tiny)] font-[number:var(--weight-title)] text-text-primary">Tool</span>
            <strong>{{ props.toolProvenance.toolName }}</strong>
          </div>
          <pre class="mt-[var(--space-2)] mb-0 mx-0 p-[var(--space-3)] font-mono text-[length:var(--text-tiny)] leading-[1.5] text-text-secondary bg-[var(--color-bg-app)] rounded-[var(--radius-md)] whitespace-pre-wrap break-words max-h-[300px] overflow-y-auto">{{ formatJson(props.toolProvenance.resolvedArgs ?? null) }}</pre>
          <pre v-if="props.toolProvenance.responseSummary" class="mt-[var(--space-2)] mb-0 mx-0 p-[var(--space-3)] font-mono text-[length:var(--text-tiny)] leading-[1.5] text-text-secondary bg-[var(--color-bg-app)] rounded-[var(--radius-md)] whitespace-pre-wrap break-words max-h-[300px] overflow-y-auto">{{ props.toolProvenance.responseSummary }}</pre>
        </AccordionContent>
      </AccordionItem>
    </Accordion>

    <div
      v-if="!props.assembledPrompt && !props.thinkingTrace && !props.workflowSteps && !props.nodeOutput && !props.toolProvenance"
      class="p-[var(--space-8)] text-center text-text-secondary text-[length:var(--text-caption)]"
    >
      This node hasn't produced any output yet.
    </div>
  </div>
</template>
