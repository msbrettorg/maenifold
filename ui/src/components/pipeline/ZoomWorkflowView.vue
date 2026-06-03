<script setup lang="ts">
// SemanticZoom content renderer: workflow steps as a vertical timeline. Active
// steps get a subtle pulsing marker; completed / failed steps map onto the
// shared node-status color tokens.
import type { WorkflowStepEntry } from '../../lib/pipeline-types';

const props = defineProps<{ steps: WorkflowStepEntry[] }>();

function statusClass(status: string): string {
  const key = status.toLowerCase();
  if (key === 'active' || key === 'running') return 'is-active';
  if (key === 'completed' || key === 'succeeded' || key === 'done') return 'is-completed';
  if (key === 'failed' || key === 'stuck' || key === 'error') return 'is-stuck';
  if (key === 'abandoned' || key === 'skipped' || key === 'cancelled') return 'is-abandoned';
  return 'is-pending';
}

function isActive(status: string): boolean {
  const key = status.toLowerCase();
  return key === 'active' || key === 'running';
}
</script>

<template>
  <ol class="zwf">
    <li
      v-for="(step, index) in props.steps"
      :key="index"
      class="zwf-step"
      :class="statusClass(step.status)"
    >
      <span class="zwf-rail" aria-hidden="true">
        <span class="zwf-dot" :class="{ pulse: isActive(step.status) }" />
      </span>
      <div class="zwf-content">
        <header class="zwf-head">
          <span class="zwf-name">{{ step.stepName }}</span>
          <span class="zwf-status">{{ step.status }}</span>
        </header>
        <pre v-if="step.output" class="zwf-output">{{ step.output }}</pre>
      </div>
    </li>
  </ol>
</template>

<style scoped>
.zwf {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.zwf-step {
  display: flex;
  gap: 0.65rem;
  --zwf-marker: var(--node-status-pending);
}

.zwf-step.is-active {
  --zwf-marker: var(--node-status-active);
}
.zwf-step.is-completed {
  --zwf-marker: var(--node-status-completed);
}
.zwf-step.is-stuck {
  --zwf-marker: var(--node-status-stuck);
}
.zwf-step.is-abandoned {
  --zwf-marker: var(--node-status-abandoned);
}

.zwf-rail {
  position: relative;
  display: flex;
  justify-content: center;
  width: 14px;
  flex-shrink: 0;
}

.zwf-rail::before {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: var(--border-color);
}

.zwf-step:first-child .zwf-rail::before {
  top: 8px;
}

.zwf-step:last-child .zwf-rail::before {
  bottom: calc(100% - 12px);
}

.zwf-dot {
  position: relative;
  z-index: 1;
  margin-top: 4px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--zwf-marker);
}

.zwf-dot.pulse {
  animation: zwf-pulse 1.4s ease-out infinite;
}

@keyframes zwf-pulse {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--node-status-active) 65%, transparent);
  }
  70% {
    box-shadow: 0 0 0 6px color-mix(in srgb, var(--node-status-active) 0%, transparent);
  }
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--node-status-active) 0%, transparent);
  }
}

.zwf-content {
  flex: 1;
  min-width: 0;
  padding-bottom: 0.7rem;
}

.zwf-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.zwf-name {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-primary);
}

.zwf-status {
  flex-shrink: 0;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--zwf-marker);
  font-weight: 700;
}

.zwf-output {
  margin: 0.35rem 0 0;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.74rem;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
