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
  <!-- audit-ok: dynamic-binding (border color reflects node status) -->
  <div
    class="workflow-node-card w-80 rounded-[var(--radius)] border-l-[3px] border-l-[var(--color-status-active)] py-3.5 px-4 flex flex-col gap-2"
    role="listitem"
    :style="{ borderLeftColor: borderColor }"
  >
    <!-- ── Header ── -->
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <component :is="workflowIcon" class="workflow-emoji size-4 shrink-0" :stroke-width="1.5" />
        <div class="flex flex-col min-w-0">
          <span class="workflow-name [font-weight:var(--weight-title)] text-[length:var(--text-caption)] whitespace-nowrap overflow-hidden text-ellipsis">{{ node.workflowName || node.workflowId }}</span>
          <span class="session-id [font-family:var(--font-code)] text-[length:var(--text-tiny)] whitespace-nowrap" :title="node.sessionId">{{ shortSessionId }}</span>
        </div>
      </div>
      <component :is="statusIcon(node.status)" class="status-badge size-4 shrink-0" :stroke-width="1.5" :title="node.status" />
    </div>

    <!-- ── Progress bar ── -->
    <div class="h-1 rounded bg-[color-mix(in_srgb,var(--color-bg-surface)_72%,var(--color-bg-app))] overflow-hidden" :title="`${completedSteps}/${totalSteps} steps`">
      <!-- audit-ok: dynamic-binding (progress bar width computed from runtime state) -->
      <div class="h-full bg-[var(--color-status-completed)] rounded transition-[width] duration-300 ease-in-out" :style="{ width: `${progressPercent}%` }" />
    </div>
    <div class="text-[length:var(--text-tiny)] text-right -mt-1">
      {{ completedSteps }}/{{ totalSteps }} steps
    </div>

    <!-- ── Step list ── -->
    <ul class="list-none p-0 m-0 flex flex-col gap-0">
      <WorkflowStepDetail
        v-for="step in node.steps"
        :key="step.stepNumber"
        :step="step"
        :expanded="isStepExpanded(step.stepNumber)"
        @toggle="toggleStep(step.stepNumber)"
      />
    </ul>

    <!-- ── Timing footer ── -->
    <div class="flex gap-4 pt-1.5 border-t border-t-[var(--workflow-card-divider)]">
      <span class="flex flex-col gap-0.5">
        <span class="text-[length:var(--text-tiny)] tracking-[0.05em]">Started</span>
        <span class="text-[length:var(--text-tiny)]">{{ startedFormatted }}</span>
      </span>
      <span v-if="durationFormatted" class="flex flex-col gap-0.5">
        <span class="text-[length:var(--text-tiny)] tracking-[0.05em]">Duration</span>
        <span class="text-[length:var(--text-tiny)]">{{ durationFormatted }}</span>
      </span>
    </div>
  </div>
</template>

<!-- audit-ok: dynamic-binding (color-mix custom properties used throughout card) -->
<style scoped>
.workflow-node-card {
  --workflow-card-text: color-mix(in srgb, var(--color-text-primary) 84%, var(--color-node-workflow));
  --workflow-card-heading: color-mix(in srgb, var(--color-text-primary) 92%, var(--color-node-workflow));
  --workflow-card-muted: color-mix(in srgb, var(--color-text-primary) 54%, var(--color-text-secondary));
  --workflow-card-subtle: color-mix(in srgb, var(--color-text-primary) 40%, var(--color-text-secondary));
  --workflow-card-divider: color-mix(in srgb, var(--color-border-subtle) 30%, var(--color-bg-surface));
  background: color-mix(in srgb, var(--color-bg-surface) 88%, var(--color-bg-app));
  color: var(--workflow-card-text);
  box-shadow: var(--shadow-8);
}

.workflow-emoji {
  color: var(--workflow-card-muted);
}

.workflow-name {
  color: var(--workflow-card-heading);
}

.session-id,
.status-badge {
  color: var(--workflow-card-muted);
}

.card-footer .timing-label {
  color: var(--workflow-card-subtle);
}

.card-footer .timing-value {
  color: var(--workflow-card-muted);
}
</style>
