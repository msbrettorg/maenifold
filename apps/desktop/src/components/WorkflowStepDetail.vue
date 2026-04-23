<script setup lang="ts">
import { Brain } from 'lucide-vue-next';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import type { WorkflowStep } from '../lib/workflow-types';

defineProps<{
  step: WorkflowStep;
  expanded: boolean;
}>();

const stepTitleId = `step-title-${Math.random().toString(36).slice(2, 10)}`

const emit = defineEmits<{
  toggle: [];
}>();

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

function renderMarkdownWithDOMPurify(text: string | undefined): string {
  if (!text) return '';
  const withPills = text.replace(/\[\[([^\]]+)\]\]/g, (_match, label: string) => {
    const escaped = label.replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return `<span class="wiki-pill">${escaped}</span>`;
  });
  const result = marked.parse(withPills, { async: false });
  return DOMPurify.sanitize(typeof result === 'string' ? result : '');
}
</script>

<template>
  <li
    class="step-row flex flex-col rounded overflow-hidden"
    role="article"
    :aria-labelledby="stepTitleId"
    :class="{ 'has-response': !!step.response, 'has-submachine': step.requiresSequentialThinking }"
  >
    <!-- Main step row -->
    <div
      class="step-main flex items-center gap-1.5 h-7 px-1 rounded"
      :class="{ clickable: !!step.response }"
      @click="step.response ? emit('toggle') : undefined"
    >
      <span class="step-num [font-family:var(--font-code)] text-[length:var(--text-tiny)] min-w-6 text-right">{{ String(step.stepNumber).padStart(2, '0') }}</span>
      <span :class="['step-icon text-[length:var(--text-tiny)] shrink-0', stepStatusClass(step.status)]">
        {{ stepStatusIcon(step.status) }}
      </span>
      <span :id="stepTitleId" class="step-name flex-1 whitespace-nowrap overflow-hidden text-ellipsis text-[length:var(--text-caption)]">{{ step.name }}</span>
      <span v-if="step.requiresSequentialThinking" class="submachine-badge text-[length:var(--text-tiny)] shrink-0" title="Sequential thinking"><Brain class="inline size-3.5" :stroke-width="1.5" /></span>
      <span v-if="step.respondedAt" class="step-time text-[length:var(--text-tiny)] shrink-0 whitespace-nowrap">{{ relativeTime(step.respondedAt) }}</span>
      <span v-if="step.response" class="expand-chevron text-[length:var(--text-tiny)] shrink-0 whitespace-nowrap">{{ expanded ? '▲' : '▼' }}</span>
    </div>

    <!-- Submachine connector -->
    <div v-if="step.requiresSequentialThinking && step.submachineSessionId" class="submachine-connector flex items-center gap-1 pl-9 mb-0.5">
      <span class="connector-line text-[var(--color-status-active)] text-[length:var(--text-tiny)] leading-none">│</span>
      <span class="connector-label [font-family:var(--font-code)] text-[length:var(--text-tiny)] whitespace-nowrap overflow-hidden text-ellipsis">→ thinking: {{ step.submachineSessionId }}</span>
    </div>

    <!-- Expanded response -->
    <div
      v-if="expanded && step.response"
      class="step-response py-2 px-3 mt-1 mb-1.5 ml-9 rounded text-[length:var(--text-caption)] leading-[1.55] overflow-x-auto"
      v-html="renderMarkdownWithDOMPurify(step.response)"
    />
  </li>
</template>

<!-- audit-ok: max-style-rules — :deep() rules for sanitized v-html markdown content cannot be expressed as Tailwind utilities -->
<style scoped>
.step-row {
  --workflow-step-primary: color-mix(in srgb, var(--color-text-primary) 84%, var(--color-node-workflow));
  --workflow-step-muted: color-mix(in srgb, var(--color-text-primary) 58%, var(--color-text-secondary));
  --workflow-step-subtle: color-mix(in srgb, var(--color-text-primary) 40%, var(--color-text-secondary));
  --workflow-step-link: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-status-active));
  --workflow-step-response: color-mix(in srgb, var(--color-text-primary) 74%, var(--color-node-workflow));
  --workflow-step-border: color-mix(in srgb, var(--color-border-subtle) 32%, var(--color-bg-surface));
}

.step-main.clickable {
  cursor: pointer;
}

.step-main.clickable:hover {
  background: color-mix(in srgb, var(--color-bg-surface) 80%, var(--color-bg-app));
}

.step-num {
  color: var(--workflow-step-subtle);
}

.step-done {
  color: var(--color-status-completed);
}

.step-active {
  color: var(--color-status-active);
}

.step-pending {
  color: var(--workflow-step-subtle);
}

.step-name {
  color: var(--workflow-step-primary);
}

.step-time,
.expand-chevron {
  color: var(--workflow-step-muted);
}

.connector-label {
  color: color-mix(in srgb, var(--workflow-step-link) 56%, var(--workflow-step-muted));
}

.step-response {
  background: color-mix(in srgb, var(--color-bg-surface) 94%, var(--color-bg-app));
  color: var(--workflow-step-response);
}

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
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  background: color-mix(in srgb, var(--color-bg-surface) 88%, var(--color-bg-app));
  padding: 0.1em 0.3em;
  border-radius: 4px;
  color: var(--workflow-step-link);
}

.step-response :deep(pre) {
  background: color-mix(in srgb, var(--color-bg-surface) 98%, var(--color-bg-app));
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
  font-size: var(--text-caption);
  overflow-x: auto;
  display: block;
}

.step-response :deep(th),
.step-response :deep(td) {
  border: 1px solid var(--workflow-step-border);
  padding: 0.5rem 0.75rem;
  text-align: left;
}

.step-response :deep(th) {
  background: var(--color-bg-surface);
  font-weight: var(--weight-title);
}

.step-response :deep(tr:nth-child(even)) {
  background: var(--color-bg-elevated);
}

.step-response :deep(ul),
.step-response :deep(ol) {
  margin: 0.25rem 0;
  padding-left: 1.25rem;
}

.step-response :deep(li) {
  margin: 0.125rem 0;
}

.step-response :deep(.wiki-pill) {
  display: inline-block;
  background: color-mix(in srgb, var(--color-brand) 18%, var(--color-bg-surface));
  color: var(--workflow-step-link);
  border-radius: 4px;
  padding: 0.05em 0.4em;
  font-size: var(--text-caption);
  font-weight: var(--weight-display);
  white-space: nowrap;
}
</style>
