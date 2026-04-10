<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WorkflowSession } from '../lib/workflow-types';
import WorkflowStepDetail from './WorkflowStepDetail.vue';

const props = defineProps<{
  node: WorkflowSession;
}>();

// ─── Collapsed step tracking ─────────────────────────────────────────────────
const expandedSteps = ref<Set<number>>(new Set());

function toggleStep(stepNumber: number): void {
  if (expandedSteps.value.has(stepNumber)) {
    expandedSteps.value.delete(stepNumber);
  } else {
    expandedSteps.value.add(stepNumber);
  }
}

function isStepExpanded(stepNumber: number): boolean {
  return expandedSteps.value.has(stepNumber);
}

// ─── Status helpers ──────────────────────────────────────────────────────────
const statusBadge = computed(() => {
  switch (props.node.status) {
    case 'completed': return '✅';
    case 'abandoned': return '⚠️';
    case 'active':    return '🔄';
    default:          return '🔄';
  }
});

const borderColor = computed(() => {
  switch (props.node.status) {
    case 'completed': return 'var(--color-status-completed)';
    case 'abandoned': return 'var(--color-status-stuck)';
    case 'active':    return 'var(--color-status-active)';
    default:          return 'var(--color-status-active)';
  }
});

// ─── Session ID truncation ────────────────────────────────────────────────────
const shortSessionId = computed(() => {
  const id = props.node.sessionId;
  if (id.length <= 16) return id;
  return id.slice(0, 8) + '…' + id.slice(-6);
});

// ─── Progress bar ─────────────────────────────────────────────────────────────
const completedSteps = computed(() =>
  props.node.steps.filter((s) => s.status === 'completed').length
);

const totalSteps = computed(() => props.node.steps.length);

const progressPercent = computed(() =>
  totalSteps.value === 0 ? 0 : Math.round((completedSteps.value / totalSteps.value) * 100)
);

// ─── Workflow name / emoji ────────────────────────────────────────────────────
const workflowEmoji = computed(() => {
  const name = props.node.workflowName ?? props.node.workflowId;
  if (name.includes('dispatch'))  return '📡';
  if (name.includes('think'))     return '🧠';
  if (name.includes('research'))  return '🔬';
  if (name.includes('review'))    return '🔍';
  if (name.includes('plan'))      return '📐';
  return '⚙️';
});

// ─── Timing ──────────────────────────────────────────────────────────────────
const startedFormatted = computed(() => formatAbsolute(props.node.started));

const durationFormatted = computed((): string | null => {
  if (props.node.status !== 'completed') return null;
  const lastStep = [...props.node.steps]
    .filter((s) => s.respondedAt)
    .sort((a, b) => new Date(b.respondedAt!).getTime() - new Date(a.respondedAt!).getTime())[0];
  if (!lastStep?.respondedAt) return null;
  const ms = new Date(lastStep.respondedAt).getTime() - new Date(props.node.started).getTime();
  return formatDuration(ms);
});

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return rem > 0 ? `${m}m ${rem}s` : `${m}m`;
}

function formatAbsolute(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}
</script>

<template>
  <div
    class="workflow-node-card"
    :style="{ borderLeftColor: borderColor }"
  >
    <!-- ── Header ── -->
    <div class="card-header">
      <div class="header-left">
        <span class="workflow-emoji">{{ workflowEmoji }}</span>
        <div class="header-titles">
          <span class="workflow-name">{{ node.workflowName || node.workflowId }}</span>
          <span class="session-id" :title="node.sessionId">{{ shortSessionId }}</span>
        </div>
      </div>
      <span class="status-badge" :title="node.status">{{ statusBadge }}</span>
    </div>

    <!-- ── Progress bar ── -->
    <div class="progress-track" :title="`${completedSteps}/${totalSteps} steps`">
      <div class="progress-fill" :style="{ width: `${progressPercent}%` }" />
    </div>
    <div class="progress-label">
      {{ completedSteps }}/{{ totalSteps }} steps
    </div>

    <!-- ── Step list ── -->
    <ul class="step-list">
      <WorkflowStepDetail
        v-for="step in node.steps"
        :key="step.stepNumber"
        :step="step"
        :expanded="isStepExpanded(step.stepNumber)"
        @toggle="toggleStep(step.stepNumber)"
      />
    </ul>

    <!-- ── Timing footer ── -->
    <div class="card-footer">
      <span class="timing-item">
        <span class="timing-label">Started</span>
        <span class="timing-value">{{ startedFormatted }}</span>
      </span>
      <span v-if="durationFormatted" class="timing-item">
        <span class="timing-label">Duration</span>
        <span class="timing-value">{{ durationFormatted }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
/* ── Card shell ─────────────────────────────────────────────────────────────── */
.workflow-node-card {
  --workflow-card-text: color-mix(in srgb, var(--text-on-accent) 84%, var(--color-node-workflow));
  --workflow-card-heading: color-mix(in srgb, var(--text-on-accent) 92%, var(--color-node-workflow));
  --workflow-card-muted: color-mix(in srgb, var(--text-on-accent) 54%, var(--text-secondary));
  --workflow-card-subtle: color-mix(in srgb, var(--text-on-accent) 40%, var(--text-secondary));
  --workflow-card-divider: color-mix(in srgb, var(--border-default) 30%, var(--bg-code));
  width: 320px;
  background: color-mix(in srgb, var(--bg-code) 88%, var(--bg-main));
  border-radius: 8px;
  border-left: 3px solid var(--color-status-active);
  padding: 0.875rem 1rem;
  color: var(--workflow-card-text);
  font-size: var(--text-sm);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--bg-main) 35%, transparent);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* ── Header ─────────────────────────────────────────────────────────────────── */
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.workflow-emoji {
  font-size: var(--text-base);
  flex-shrink: 0;
}

.header-titles {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.workflow-name {
  font-weight: 600;
  font-size: var(--text-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--workflow-card-heading);
}

.session-id {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--workflow-card-muted);
  white-space: nowrap;
}

.status-badge {
  font-size: var(--text-base);
  flex-shrink: 0;
}

/* ── Progress bar ───────────────────────────────────────────────────────────── */
.progress-track {
  height: 4px;
  background: color-mix(in srgb, var(--bg-code) 72%, var(--bg-main));
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--bg-success);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.progress-label {
  font-size: var(--text-xs);
  color: var(--workflow-card-muted);
  text-align: right;
  margin-top: -0.25rem;
}

/* ── Step list ──────────────────────────────────────────────────────────────── */
.step-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0;
}

/* ── Footer timing ──────────────────────────────────────────────────────────── */
.card-footer {
  display: flex;
  gap: 1rem;
  padding-top: 0.375rem;
  border-top: 1px solid var(--workflow-card-divider);
}

.timing-item {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.timing-label {
  font-size: var(--text-xs);
  color: var(--workflow-card-subtle);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.timing-value {
  font-size: var(--text-xs);
  color: var(--workflow-card-muted);
}
</style>
