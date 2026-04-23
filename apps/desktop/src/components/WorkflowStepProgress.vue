<script setup lang="ts">
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
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
  pending: 'bg-[var(--color-status-pending)]',
  completed: 'bg-[var(--color-status-completed)]',
  active: 'bg-[var(--color-status-active)]',
  failed: 'bg-[var(--color-status-stuck)]',
}

// Use color-mix for line + badge background/border via inline style binding
function badgeStyle(s: string): Record<string, string> {
  const tokenMap: Record<StatusKey, string> = {
    completed: 'var(--color-status-completed)',
    active: 'var(--color-status-active)',
    failed: 'var(--color-status-stuck)',
    pending: 'var(--color-status-pending)',
  }
  const token = tokenMap[statusKey(s)]
  return {
    color: token,
    background: `color-mix(in srgb, ${token} 12%, var(--color-bg-app))`,
    borderColor: `color-mix(in srgb, ${token} 30%, transparent)`,
  }
}

function stepLineStyle(s: string): Record<string, string> {
  if (isCompleted(s)) {
    return { background: 'color-mix(in srgb, var(--color-status-completed) 40%, var(--color-border-subtle))' }
  }
  return { background: 'var(--color-border-subtle)' }
}
</script>

<template>
  <section class="flex flex-col gap-3" aria-label="Workflow steps">
    <div class="flex items-start justify-between gap-3">
      <h3 class="m-0 text-[length:var(--text-body-sm)] text-[var(--color-text-primary)]">Workflow steps</h3>
      <span class="text-[var(--color-text-secondary)] text-[length:var(--text-tiny)]">{{ steps.length }} steps</span>
    </div>

    <ol class="list-none m-0 p-0 flex flex-col">
      <li
        v-for="(step, index) in steps"
        :key="`${step.step_name}-${index}`"
        class="flex gap-3 min-h-10"
      >
        <div class="flex flex-col items-center shrink-0 w-5 pt-[0.15rem]" aria-hidden="true">
          <span class="relative w-[0.625rem] h-[0.625rem] rounded-[var(--radius)] shrink-0" :class="dotBg[statusKey(step.status)]">
            <!-- audit-ok: keyframe-animation — pulse-ring requires @keyframes -->
            <span v-if="isActive(step.status)" class="step-dot__pulse" />
          </span>
          <!-- audit-ok: dynamic-binding — line color depends on completion status with color-mix() -->
          <span v-if="index < steps.length - 1" class="flex-1 w-0.5 min-h-3" :style="stepLineStyle(step.status)" />
        </div>

        <div class="flex-1 flex flex-col gap-1 pb-2 min-w-0">
          <div class="flex items-center justify-between gap-2">
            <span
              class="text-[length:var(--text-caption)]"
              :class="statusKey(step.status) === 'pending' ? 'text-[var(--color-text-secondary)]' : 'text-[var(--color-text-primary)]'"
            >{{ formatStepName(step.step_name) }}</span>
            <!-- audit-ok: dynamic-binding — badge color/background/border depend on status with color-mix() -->
            <span
              class="inline-flex items-center px-[0.45rem] py-[0.1rem] rounded-[4px] border border-transparent text-[length:var(--text-tiny)] [font-weight:var(--weight-title)] whitespace-nowrap"
              :style="badgeStyle(step.status)"
            >
              {{ step.status }}
            </span>
          </div>
          <Accordion v-if="step.output" type="multiple" class="text-[length:var(--text-caption)] text-[var(--color-text-secondary)]">
            <AccordionItem :value="`step-output-${step.step_name}`">
              <AccordionTrigger>Output</AccordionTrigger>
              <AccordionContent>
                <pre class="mt-1 mb-0 p-2 bg-[var(--color-bg-app)] border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] whitespace-pre-wrap break-words [font-family:var(--font-code)] text-[length:var(--text-tiny)] text-[var(--color-text-primary)] overflow-x-auto">{{ step.output }}</pre>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </div>
      </li>
    </ol>
  </section>
</template>

<style scoped>
/* audit-ok: keyframe-animation — pulse ring animation for active step indicator */
.step-dot__pulse {
  position: absolute;
  inset: -3px;
  border-radius: var(--radius);
  border: 2px solid var(--color-status-active);
  animation: pulse-ring 1.4s ease-in-out infinite;
}
@keyframes pulse-ring {
  0% { opacity: 1; transform: scale(1); }
  100% { opacity: 0; transform: scale(1.8); }
}
</style>
