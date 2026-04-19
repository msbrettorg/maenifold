<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { formatStageLabel, gateStageFamily, getTaskPathHint } from '../lib/gate-display';

const props = defineProps<{
  task: GateTask;
  processLabel: string;
}>();

const fileLabel = computed(() => getTaskPathHint(props.task.path));
const stateTone = computed(() => gateStageFamily(props.task.location === 'done' ? 'done' : props.task.state));
</script>

<template>
  <dl class="gate-detail-meta" aria-label="Task metadata">
    <div>
      <dt>Process</dt>
      <dd>
        <span class="gate-detail-meta__badge gate-detail-meta__badge--process">{{ processLabel }}</span>
      </dd>
    </div>
    <div>
      <dt>State</dt>
      <dd>
        <span :class="['gate-detail-meta__badge', 'gate-detail-meta__badge--state', `gate-detail-meta__badge--${stateTone}`]">
          {{ formatStageLabel(task.location === 'done' ? 'done' : task.state) }}
        </span>
      </dd>
    </div>
    <div>
      <dt>Location</dt>
      <dd>{{ formatStageLabel(task.location) }}</dd>
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
      <dt>File</dt>
      <dd :title="fileLabel">{{ fileLabel }}</dd>
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
  color: var(--color-text-secondary);
  font-size: var(--text-tiny);
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
  border-radius: 4px;
  background: var(--surface-highest);
  color: var(--color-text-primary);
  font-family: var(--font-code);
  font-size: var(--text-2xs);
  font-weight: var(--weight-title);
  line-height: 1.4;
}

.gate-detail-meta__badge--process { background: var(--outline-variant); }
.gate-detail-meta__badge--pending { background: var(--text-dim); }
.gate-detail-meta__badge--swe { background: var(--blue); }
.gate-detail-meta__badge--red-team { background: var(--amber); }
.gate-detail-meta__badge--blue-team,
.gate-detail-meta__badge--done { background: var(--sage); }
.gate-detail-meta__badge--failed { background: var(--terracotta); }

.gate-detail-meta__path {
  grid-column: 1 / -1;
}
</style>
