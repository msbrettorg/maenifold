<script setup lang="ts">
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Badge } from '@/components/ui/badge';
import type { WorkflowStepEntry } from '../lib/pipeline-types';

defineProps<{
  steps: WorkflowStepEntry[];
}>();

function formatStepName(name: string): string {
  return name
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (l) => l.toUpperCase());
}

function isCompleted(status: string): boolean {
  return /complete|done|success/i.test(status);
}

function isActive(status: string): boolean {
  return /active|running|in[- ]?progress/i.test(status);
}

function isFailed(status: string): boolean {
  return /fail|error|stuck|abandon/i.test(status);
}

type StatusKey = 'completed' | 'active' | 'failed' | 'pending';

function statusKey(status: string): StatusKey {
  if (isCompleted(status)) return 'completed';
  if (isActive(status)) return 'active';
  if (isFailed(status)) return 'failed';
  return 'pending';
}

const dotBg: Record<StatusKey, string> = {
  pending: 'bg-status-pending',
  completed: 'bg-status-completed',
  active: 'bg-status-active',
  failed: 'bg-status-stuck',
};

const badgeClass: Record<StatusKey, string> = {
  pending: 'border-border-subtle text-text-secondary bg-bg-surface',
  completed: 'border-status-completed/30 text-status-completed bg-status-completed-15',
  active: 'border-status-active/30 text-status-active bg-status-active-15',
  failed: 'border-status-stuck/30 text-status-stuck bg-status-stuck-15',
};

function lineClass(s: string): string {
  return isCompleted(s) ? 'bg-status-completed/40' : 'bg-border-subtle';
}
</script>

<template>
  <section class="flex flex-col gap-3" aria-label="Workflow steps">
    <div class="flex items-start justify-between gap-3">
      <h3 class="m-0 text-sm text-text-primary">Workflow steps</h3>
      <span class="text-text-secondary text-xs">{{ steps.length }} steps</span>
    </div>

    <ol class="list-none m-0 p-0 flex flex-col">
      <li
        v-for="(step, index) in steps"
        :key="`${step.step_name}-${index}`"
        class="flex gap-3 min-h-10"
      >
        <div class="flex flex-col items-center shrink-0 w-5 pt-0.5" aria-hidden="true">
          <span
            class="relative w-2.5 h-2.5 rounded shrink-0"
            :class="[dotBg[statusKey(step.status)], isActive(step.status) ? 'animate-pulse' : '']"
          />
          <span
            v-if="index < steps.length - 1"
            class="flex-1 w-0.5 min-h-3"
            :class="lineClass(step.status)"
          />
        </div>

        <div class="flex-1 flex flex-col gap-1 pb-2 min-w-0">
          <div class="flex items-center justify-between gap-2">
            <span
              class="text-xs"
              :class="statusKey(step.status) === 'pending' ? 'text-text-secondary' : 'text-text-primary'"
            >{{ formatStepName(step.step_name) }}</span>
            <Badge
              variant="outline"
              :class="['whitespace-nowrap', badgeClass[statusKey(step.status)]]"
            >
              {{ step.status }}
            </Badge>
          </div>
          <Accordion v-if="step.output" type="multiple" class="text-xs text-text-secondary">
            <AccordionItem :value="`step-output-${step.step_name}`">
              <AccordionTrigger>Output</AccordionTrigger>
              <AccordionContent>
                <pre class="mt-1 mb-0 p-2 bg-bg-app border border-border-subtle rounded-sm whitespace-pre-wrap break-words font-mono text-xs text-text-primary overflow-x-auto">{{ step.output }}</pre>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </div>
      </li>
    </ol>
  </section>
</template>
