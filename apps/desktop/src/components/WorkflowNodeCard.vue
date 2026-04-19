<script setup lang="ts">
import { Brain, FlaskConical, Search, Ruler, Settings, RefreshCw, AlertTriangle } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import type { Component } from 'vue';
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
function statusIcon(status: string): Component {
  switch (status) {
    case 'abandoned': return AlertTriangle;
    case 'active': return RefreshCw;
    default: return RefreshCw;
  }
}

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
function nodeTypeIcon(name: string): Component {
  if (name.includes('think')) return Brain;
  if (name.includes('research')) return FlaskConical;
  if (name.includes('review')) return Search;
  if (name.includes('plan')) return Ruler;
  return Settings;
}

const workflowIcon = computed(() => {
  const name = props.node.workflowName ?? props.node.workflowId;
  return nodeTypeIcon(name);
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
    role="listitem"
    :style="{ borderLeftColor: borderColor }"
  >
    <!-- ── Header ── -->
    <div class="card-header">
      <div class="header-left">
        <component :is="workflowIcon" class="workflow-emoji size-4" :stroke-width="1.5" />
        <div class="header-titles">
          <span class="workflow-name">{{ node.workflowName || node.workflowId }}</span>
          <span class="session-id" :title="node.sessionId">{{ shortSessionId }}</span>
        </div>
      </div>
      <component :is="statusIcon(node.status)" class="status-badge size-4" :stroke-width="1.5" :title="node.status" />
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
  --workflow-card-text: color-mix(in srgb, var(--color-text-primary) 84%, var(--color-node-workflow));
  --workflow-card-heading: color-mix(in srgb, var(--color-text-primary) 92%, var(--color-node-workflow));
  --workflow-card-muted: color-mix(in srgb, var(--color-text-primary) 54%, var(--color-text-secondary));
  --workflow-card-subtle: color-mix(in srgb, var(--color-text-primary) 40%, var(--color-text-secondary));
  --workflow-card-divider: color-mix(in srgb, var(--color-border-subtle) 30%, var(--color-bg-surface));
  width: 320px;
  background: color-mix(in srgb, var(--color-bg-surface) 88%, var(--color-bg-app));
  border-radius: var(--radius);
  border-left: 3px solid var(--color-status-active);
  padding: 0.875rem 1rem;
  color: var(--workflow-card-text);
  font-size: var(--text-caption);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-bg-app) 35%, transparent);
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
  color: var(--workflow-card-muted);
  flex-shrink: 0;
}

.header-titles {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.workflow-name {
  font-weight: var(--weight-title);
  font-size: var(--text-caption);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--workflow-card-heading);
}

.session-id {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  color: var(--workflow-card-muted);
  white-space: nowrap;
}

.status-badge {
  color: var(--workflow-card-muted);
  flex-shrink: 0;
}

/* ── Progress bar ───────────────────────────────────────────────────────────── */
.progress-track {
  height: 4px;
  background: color-mix(in srgb, var(--color-bg-surface) 72%, var(--color-bg-app));
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--color-status-completed);
  border-radius: 4px;
  transition: width 0.3s ease;
}

.progress-label {
  font-size: var(--text-tiny);
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
  font-size: var(--text-tiny);
  color: var(--workflow-card-subtle);
  letter-spacing: 0.05em;
}

.timing-value {
  font-size: var(--text-tiny);
  color: var(--workflow-card-muted);
}
</style>
