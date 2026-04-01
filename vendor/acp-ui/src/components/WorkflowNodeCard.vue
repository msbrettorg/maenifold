<script setup lang="ts">
import { computed, ref } from 'vue';
import { marked } from 'marked';
import type { WorkflowSession } from '../lib/workflow-types';

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
    case 'completed':  return '✅';
    case 'abandoned':  return '⚠️';
    case 'active':     return '🔄';
    default:           return '🔄';
  }
});

const borderColor = computed(() => {
  switch (props.node.status) {
    case 'completed': return '#22c55e'; // green
    case 'abandoned': return '#f59e0b'; // amber
    case 'active':    return '#3b82f6'; // blue
    default:          return '#3b82f6';
  }
});

function stepStatusIcon(status: 'pending' | 'active' | 'completed'): string {
  switch (status) {
    case 'completed': return '●';
    case 'active':    return '◐';
    case 'pending':   return '○';
    default:          return '○';
  }
}

function stepStatusClass(status: 'pending' | 'active' | 'completed'): string {
  switch (status) {
    case 'completed': return 'step-done';
    case 'active':    return 'step-active';
    case 'pending':   return 'step-pending';
    default:          return 'step-pending';
  }
}

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

function relativeTime(iso: string | undefined): string {
  if (!iso) return '';
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 0) return 'just now';
  const s = Math.floor(diff / 1000);
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

// ─── Workflow name / emoji ────────────────────────────────────────────────────
const workflowEmoji = computed(() => {
  // derive a consistent emoji from workflow name if none embedded
  const name = props.node.workflowName ?? props.node.workflowId;
  if (name.includes('dispatch'))  return '📡';
  if (name.includes('think'))     return '🧠';
  if (name.includes('research'))  return '🔬';
  if (name.includes('review'))    return '🔍';
  if (name.includes('plan'))      return '📐';
  return '⚙️';
});

// ─── Markdown + WikiLinks ─────────────────────────────────────────────────────
function renderMarkdown(text: string | undefined): string {
  if (!text) return '';
  // Replace [[WikiLinks]] with styled pill spans before markdown
  const withPills = text.replace(/\[\[([^\]]+)\]\]/g, (_match, label: string) => {
    const escaped = label.replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return `<span class="wiki-pill">${escaped}</span>`;
  });
  const result = marked.parse(withPills);
  return typeof result === 'string' ? result : '';
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
      <li
        v-for="step in node.steps"
        :key="step.stepNumber"
        class="step-row"
        :class="{ 'has-response': !!step.response, 'has-submachine': step.requiresSequentialThinking }"
      >
        <!-- Main step row -->
        <div
          class="step-main"
          :class="{ clickable: !!step.response }"
          @click="step.response ? toggleStep(step.stepNumber) : undefined"
        >
          <span class="step-num">{{ String(step.stepNumber).padStart(2, '0') }}</span>
          <span :class="['step-icon', stepStatusClass(step.status)]">
            {{ stepStatusIcon(step.status) }}
          </span>
          <span class="step-name">{{ step.name }}</span>
          <span v-if="step.requiresSequentialThinking" class="submachine-badge" title="Sequential thinking">🧠</span>
          <span v-if="step.respondedAt" class="step-time">{{ relativeTime(step.respondedAt) }}</span>
          <span v-if="step.response" class="expand-chevron">{{ isStepExpanded(step.stepNumber) ? '▲' : '▼' }}</span>
        </div>

        <!-- Submachine connector -->
        <div v-if="step.requiresSequentialThinking && step.submachineSessionId" class="submachine-connector">
          <span class="connector-line">│</span>
          <span class="connector-label">→ thinking: {{ step.submachineSessionId }}</span>
        </div>

        <!-- Expanded response -->
        <div
          v-if="isStepExpanded(step.stepNumber) && step.response"
          class="step-response"
          v-html="renderMarkdown(step.response)"
        />
      </li>
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
  width: 320px;
  background: #1e2028;
  border-radius: 8px;
  border-left: 3px solid #3b82f6;
  padding: 0.875rem 1rem;
  color: #d4d8e8;
  font-size: 0.8125rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
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
  font-size: 1rem;
  flex-shrink: 0;
}

.header-titles {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.workflow-name {
  font-weight: 600;
  font-size: 0.875rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: #e2e6f3;
}

.session-id {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
  color: #6b7280;
  white-space: nowrap;
}

.status-badge {
  font-size: 1rem;
  flex-shrink: 0;
}

/* ── Progress bar ───────────────────────────────────────────────────────────── */
.progress-track {
  height: 4px;
  background: #2d3144;
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #22c55e;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.progress-label {
  font-size: 0.6875rem;
  color: #6b7280;
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

.step-row {
  display: flex;
  flex-direction: column;
  border-radius: 4px;
  overflow: hidden;
}

.step-main {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  height: 28px;
  padding: 0 0.25rem;
  border-radius: 4px;
}

.step-main.clickable {
  cursor: pointer;
}

.step-main.clickable:hover {
  background: #252836;
}

.step-num {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.6875rem;
  color: #4b5563;
  min-width: 1.5rem;
  text-align: right;
}

.step-icon {
  font-size: 0.75rem;
  flex-shrink: 0;
}

.step-done   { color: #22c55e; }
.step-active { color: #3b82f6; }
.step-pending { color: #4b5563; }

.step-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 0.8rem;
  color: #c9cde0;
}

.submachine-badge {
  font-size: 0.75rem;
  flex-shrink: 0;
}

.step-time {
  font-size: 0.6875rem;
  color: #6b7280;
  white-space: nowrap;
  flex-shrink: 0;
}

.expand-chevron {
  font-size: 0.5625rem;
  color: #6b7280;
  flex-shrink: 0;
}

/* ── Submachine connector ────────────────────────────────────────────────────── */
.submachine-connector {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding-left: 2.25rem;
  margin-bottom: 0.125rem;
}

.connector-line {
  color: #3b82f6;
  font-size: 0.75rem;
  line-height: 1;
}

.connector-label {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.6375rem;
  color: #4b6a9e;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── Expanded response ───────────────────────────────────────────────────────── */
.step-response {
  padding: 0.5rem 0.75rem;
  margin: 0.25rem 0 0.375rem 2.25rem;
  background: #161820;
  border-radius: 4px;
  font-size: 0.775rem;
  color: #b8bcd0;
  line-height: 1.55;
  overflow-x: auto;
}

/* Markdown elements inside response */
.step-response :deep(p) {
  margin: 0.25rem 0;
}

.step-response :deep(p:first-child) {
  margin-top: 0;
}

.step-response :deep(p:last-child) {
  margin-bottom: 0;
}

.step-response :deep(code) {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.75rem;
  background: #1e2028;
  padding: 0.1em 0.3em;
  border-radius: 3px;
  color: #93c5fd;
}

.step-response :deep(pre) {
  background: #13141a;
  border-radius: 4px;
  padding: 0.5rem;
  overflow-x: auto;
}

.step-response :deep(pre code) {
  background: none;
  padding: 0;
}

.step-response :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
  font-size: 0.875rem;
  overflow-x: auto;
  display: block;
}

.step-response :deep(th),
.step-response :deep(td) {
  border: 1px solid var(--border-color, #3d3d3d);
  padding: 0.5rem 0.75rem;
  text-align: left;
}

.step-response :deep(th) {
  background: var(--bg-tertiary, #2d2d2d);
  font-weight: 600;
}

.step-response :deep(tr:nth-child(even)) {
  background: var(--bg-secondary, #1e1e1e);
}

.step-response :deep(ul),
.step-response :deep(ol) {
  margin: 0.25rem 0;
  padding-left: 1.25rem;
}

.step-response :deep(li) {
  margin: 0.125rem 0;
}

/* ── WikiLink pills ────────────────────────────────────────────────────────── */
.step-response :deep(.wiki-pill) {
  display: inline-block;
  background: #2d3a5a;
  color: #93c5fd;
  border-radius: 3px;
  padding: 0.05em 0.4em;
  font-size: 0.775rem;
  font-weight: 500;
  white-space: nowrap;
}

/* ── Footer timing ──────────────────────────────────────────────────────────── */
.card-footer {
  display: flex;
  gap: 1rem;
  padding-top: 0.375rem;
  border-top: 1px solid #252836;
}

.timing-item {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.timing-label {
  font-size: 0.625rem;
  color: #4b5563;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.timing-value {
  font-size: 0.7rem;
  color: #9ca3af;
}
</style>
