<script setup lang="ts">
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import type { SequentialThinkingViewNode } from '../lib/pipeline-types';

defineProps<{
  trace: NonNullable<SequentialThinkingViewNode['thinking_trace']>;
}>();

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

function formatStatus(status: string): string {
  return status
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function isRevision(status: string): boolean {
  return /revis/i.test(status);
}

function revisionTarget(status: string): number | null {
  const match = status.match(/revis(?:e|es|ed|ion)?[^\d]*(\d+)/i);
  return match ? Number(match[1]) : null;
}

function isActive(status: string): boolean {
  return /active|running|in[- ]?progress/i.test(status);
}

function statusClass(status: string): string {
  if (isRevision(status)) return 'trace-status--revision';
  if (isActive(status)) return 'trace-status--active';
  if (/complete|done/i.test(status)) return 'trace-status--completed';
  if (/stuck|abandon|fail|error/i.test(status)) return 'trace-status--warning';
  return 'trace-status--pending';
}
</script>

<template>
  <section class="trace-panel" aria-label="Thinking trace">
    <div class="trace-header">
      <h3>Thinking trace</h3>
      <span class="trace-count">{{ trace.length }} thoughts</span>
    </div>

    <ol class="trace-list">
      <li v-for="(thought, index) in trace" :key="`${thought.thought_number}-${index}`">
        <details class="trace-card" :open="index < 3">
          <summary class="trace-summary">
            <div class="trace-summary__copy">
              <span class="trace-number">Thought {{ thought.thought_number }}</span>
              <span v-if="isRevision(thought.status)" class="trace-revision">
                Revises {{ revisionTarget(thought.status) ? `#${revisionTarget(thought.status)}` : 'an earlier thought' }}
              </span>
            </div>
            <span :class="['trace-status', statusClass(thought.status)]">
              <span v-if="isActive(thought.status)" class="trace-status__dot" aria-hidden="true" />
              {{ formatStatus(thought.status) }}
            </span>
          </summary>

          <div class="trace-body">
            <div class="trace-content" v-html="renderMarkdown(thought.text)" />
          </div>
        </details>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.trace-panel { display: flex; flex-direction: column; gap: var(--space-3); }
.trace-header, .trace-summary { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-3); }
.trace-header h3 { margin: 0; font-size: var(--text-base); color: var(--text-primary); }
.trace-count, .trace-revision { color: var(--text-secondary); font-size: var(--text-xs); }
.trace-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--space-2); }
.trace-card {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  overflow: hidden;
}
.trace-summary {
  list-style: none;
  cursor: pointer;
  padding: var(--space-3);
}
.trace-summary::-webkit-details-marker { display: none; }
.trace-summary__copy { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.trace-number { font-family: var(--font-mono); font-size: var(--text-sm); color: var(--text-primary); }
.trace-status {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  border: 1px solid transparent;
  font-size: var(--text-xs);
  font-weight: 600;
  white-space: nowrap;
}
.trace-status__dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: currentColor;
}
.trace-status--active {
  color: var(--color-status-active);
  background: color-mix(in srgb, var(--color-status-active) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--color-status-active) 30%, transparent);
}
.trace-status--completed {
  color: var(--color-status-completed);
  background: color-mix(in srgb, var(--color-status-completed) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--color-status-completed) 30%, transparent);
}
.trace-status--warning {
  color: var(--color-status-stuck);
  background: color-mix(in srgb, var(--color-status-stuck) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--color-status-stuck) 30%, transparent);
}
.trace-status--revision {
  color: var(--border-focus);
  background: color-mix(in srgb, var(--border-focus) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--border-focus) 30%, transparent);
}
.trace-status--pending {
  color: var(--color-status-pending);
  background: color-mix(in srgb, var(--color-status-pending) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--color-status-pending) 30%, transparent);
}
.trace-body {
  padding: 0 var(--space-3) var(--space-3);
  border-top: 1px solid var(--border-default);
}
.trace-content {
  color: var(--text-primary);
  font-size: var(--text-sm);
  line-height: 1.6;
  overflow-wrap: anywhere;
}
.trace-content :deep(*) { margin-top: 0; }
.trace-content :deep(p:last-child) { margin-bottom: 0; }
.trace-content :deep(code) {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  background: var(--bg-main);
  border-radius: var(--radius-sm);
  padding: 0.1rem 0.3rem;
}
.trace-content :deep(pre) {
  margin: var(--space-2) 0 0;
  padding: var(--space-2);
  background: var(--bg-main);
  border-radius: var(--radius-sm);
  overflow-x: auto;
}
</style>
