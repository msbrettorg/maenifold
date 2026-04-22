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

function statusClass(status: string): string {
  if (isCompleted(status)) return 'step--completed';
  if (isActive(status)) return 'step--active';
  if (isFailed(status)) return 'step--failed';
  return 'step--pending';
}
</script>

<template>
  <section class="steps-panel" aria-label="Workflow steps">
    <div class="steps-header">
      <h3>Workflow steps</h3>
      <span class="steps-count">{{ steps.length }} steps</span>
    </div>

    <ol class="steps-timeline">
      <li
        v-for="(step, index) in steps"
        :key="`${step.step_name}-${index}`"
        :class="['step', statusClass(step.status)]"
      >
        <div class="step-indicator" aria-hidden="true">
          <span class="step-dot">
            <span v-if="isActive(step.status)" class="step-dot__pulse" />
          </span>
          <span v-if="index < steps.length - 1" class="step-line" />
        </div>

        <div class="step-content">
          <div class="step-row">
            <span class="step-name">{{ formatStepName(step.step_name) }}</span>
            <span :class="['step-badge', statusClass(step.status)]">
              {{ step.status }}
            </span>
          </div>
          <Accordion v-if="step.output" type="multiple" class="step-output">
            <AccordionItem :value="`step-output-${step.step_name}`">
              <AccordionTrigger>Output</AccordionTrigger>
              <AccordionContent>
                <pre>{{ step.output }}</pre>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </div>
      </li>
    </ol>
  </section>
</template>

<!-- audit-ok: max-style-rules -->
<style scoped>
.steps-panel { display: flex; flex-direction: column; gap: var(--space-3); }
.steps-header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-3); }
.steps-header h3 { margin: 0; font-size: var(--text-body-sm); color: var(--color-text-primary); }
.steps-count { color: var(--color-text-secondary); font-size: var(--text-tiny); }
.steps-timeline { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
.step { display: flex; gap: var(--space-3); min-height: 2.5rem; }
.step-indicator { display: flex; flex-direction: column; align-items: center; flex-shrink: 0; width: 1.25rem; padding-top: 0.15rem; }
.step-dot { position: relative; width: 0.625rem; height: 0.625rem; border-radius: var(--radius); flex-shrink: 0; background: var(--color-status-pending); }
.step--completed .step-dot { background: var(--color-status-completed); }
.step--active .step-dot    { background: var(--color-status-active); }
.step--failed .step-dot    { background: var(--color-status-stuck); }
.step-dot__pulse { position: absolute; inset: -3px; border-radius: var(--radius); border: 2px solid var(--color-status-active); animation: pulse-ring 1.4s ease-in-out infinite; }
@keyframes pulse-ring { 0% { opacity: 1; transform: scale(1); } 100% { opacity: 0; transform: scale(1.8); } }
.step-line { flex: 1; width: 2px; min-height: 0.75rem; background: var(--color-border-subtle); }
.step--completed .step-line { background: color-mix(in srgb, var(--color-status-completed) 40%, var(--color-border-subtle)); }
.step-content { flex: 1; display: flex; flex-direction: column; gap: var(--space-1); padding-bottom: var(--space-2); min-width: 0; }
.step-row { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); }
.step-name { font-size: var(--text-caption); color: var(--color-text-primary); }
.step--pending .step-name { color: var(--color-text-secondary); }
.step-badge { display: inline-flex; align-items: center; padding: 0.1rem 0.45rem; border-radius: 4px; border: 1px solid transparent; font-size: var(--text-tiny); font-weight: var(--weight-title); white-space: nowrap; }
.step-badge.step--completed { color: var(--color-status-completed); background: color-mix(in srgb, var(--color-status-completed) 12%, var(--color-bg-app)); border-color: color-mix(in srgb, var(--color-status-completed) 30%, transparent); }
.step-badge.step--active { color: var(--color-status-active); background: color-mix(in srgb, var(--color-status-active) 12%, var(--color-bg-app)); border-color: color-mix(in srgb, var(--color-status-active) 30%, transparent); }
.step-badge.step--failed { color: var(--color-status-stuck); background: color-mix(in srgb, var(--color-status-stuck) 12%, var(--color-bg-app)); border-color: color-mix(in srgb, var(--color-status-stuck) 30%, transparent); }
.step-badge.step--pending { color: var(--color-status-pending); background: color-mix(in srgb, var(--color-status-pending) 12%, var(--color-bg-app)); border-color: color-mix(in srgb, var(--color-status-pending) 30%, transparent); }
.step-output { font-size: var(--text-caption); color: var(--color-text-secondary); }
.step-output summary { cursor: pointer; font-size: var(--text-tiny); color: var(--color-text-secondary); }
.step-output pre { margin: var(--space-1) 0 0; padding: var(--space-2); background: var(--color-bg-app); border: 1px solid var(--color-border-subtle); border-radius: var(--radius-sm); white-space: pre-wrap; word-break: break-word; font-family: var(--font-code); font-size: var(--text-tiny); color: var(--color-text-primary); overflow-x: auto; }
</style>
