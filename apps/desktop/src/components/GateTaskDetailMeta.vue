<script setup lang="ts">
import type { GateTask } from '../lib/gate';

defineProps<{
  task: GateTask;
  processLabel: string;
}>();
</script>

<template>
  <dl class="gate-detail-meta" aria-label="Task metadata">
    <div>
      <dt>Process</dt>
      <dd>
        <span class="gate-detail-meta__badge">{{ processLabel }}</span>
      </dd>
    </div>
    <div>
      <dt>State</dt>
      <dd>
        <span :class="['gate-detail-meta__badge', 'gate-detail-meta__badge--state', `gate-detail-meta__badge--${task.state}`]">
          {{ task.state }}
        </span>
      </dd>
    </div>
    <div>
      <dt>Location</dt>
      <dd>{{ task.location }}</dd>
    </div>
    <div>
      <dt>Attempts</dt>
      <dd>{{ task.attempts }}</dd>
    </div>
    <div>
      <dt>Canonical ID</dt>
      <dd>{{ task.canonicalTaskId || '—' }}</dd>
    </div>
    <div class="gate-detail-meta__path">
      <dt>Path</dt>
      <dd>{{ task.path }}</dd>
    </div>
  </dl>
</template>

<style scoped>
.gate-detail-meta,
.gate-detail-meta > div {
  display: flex;
  flex-direction: column;
}

.gate-detail-meta {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(8.5rem, 1fr));
  align-items: start;
}

.gate-detail-meta dt {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.gate-detail-meta dd {
  display: flex;
  align-items: center;
  min-height: 1.75rem;
  margin-top: 0.25rem;
  word-break: break-word;
  font-family: var(--font-code);
  font-size: var(--text-caption);
}

.gate-detail-meta__badge {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  padding: 0.25rem 0.55rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--muted);
  color: var(--muted-foreground);
  font-family: var(--font-code);
  font-size: var(--text-2xs);
  font-weight: var(--weight-title);
  line-height: 1.4;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.gate-detail-meta__badge--state {
  color: var(--foreground);
}

.gate-detail-meta__badge--pending {
  border-color: color-mix(in srgb, var(--color-status-pending) 35%, var(--border));
  color: var(--color-status-pending);
}

.gate-detail-meta__badge--active {
  border-color: color-mix(in srgb, var(--color-status-active) 35%, var(--border));
  color: var(--color-status-active);
}

.gate-detail-meta__badge--completed,
.gate-detail-meta__badge--done {
  border-color: color-mix(in srgb, var(--color-status-completed) 35%, var(--border));
  color: var(--color-status-completed);
}

.gate-detail-meta__badge--abandoned,
.gate-detail-meta__badge--cancelled {
  border-color: color-mix(in srgb, var(--color-status-abandoned) 35%, var(--border));
  color: var(--color-status-abandoned);
}

.gate-detail-meta__badge--ready,
.gate-detail-meta__badge--paused {
  border-color: color-mix(in srgb, var(--color-status-ready) 35%, var(--border));
  color: var(--color-status-ready);
}

.gate-detail-meta__badge--stuck {
  border-color: color-mix(in srgb, var(--color-status-stuck) 35%, var(--border));
  color: var(--color-status-stuck);
}

.gate-detail-meta__path {
  grid-column: 1 / -1;
}
</style>
