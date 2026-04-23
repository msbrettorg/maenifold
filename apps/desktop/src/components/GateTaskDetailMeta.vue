<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { formatStageLabel, gateStageFamily, getTaskPathHint } from '../lib/gate-display';

const props = defineProps<{
  task: GateTask;
  processLabel: string;
  pipelineStale?: boolean;
}>();

const fileLabel = computed(() => getTaskPathHint(props.task.path));
const stateTone = computed(() => gateStageFamily(props.task.location === 'done' ? 'done' : props.task.state));
const pipelineStaleTitle = computed(() => props.task.issue?.trim() || 'Pipeline definition has changed since this task was created.');

function badgeBg(tone: string): string {
  if (tone === 'pending') return 'var(--text-secondary)';
  if (tone === 'swe') return 'var(--color-brand)';
  if (tone === 'red-team') return 'var(--amber)';
  if (tone === 'blue-team' || tone === 'done') return 'var(--sage)';
  if (tone === 'failed') return 'var(--terracotta)';
  return 'var(--surface-highest)';
}
</script>

<template>
  <dl
    class="grid gap-3 grid-cols-[repeat(auto-fit,minmax(8.5rem,1fr))] items-start [&>div]:flex [&>div]:flex-col [&_dt]:text-[var(--color-text-secondary)] [&_dt]:text-[length:var(--text-tiny)] [&_dd]:flex [&_dd]:items-center [&_dd]:min-h-7 [&_dd]:mt-1 [&_dd]:break-words [&_dd]:font-mono [&_dd]:text-[length:var(--text-caption)]"
    aria-label="Task metadata"
  >
    <div>
      <dt>Process</dt>
      <dd>
        <span
          class="inline-flex items-center max-w-full px-[0.55rem] py-1 rounded-[4px] text-[var(--color-text-primary)] font-mono text-[length:var(--text-2xs)] font-[var(--weight-title)] leading-[1.4] bg-[var(--outline-variant)]"
        >{{ processLabel }}</span>
        <span
          v-if="pipelineStale"
          class="ml-[0.4rem] text-[length:var(--text-caption)] cursor-help text-[var(--amber,var(--terracotta))]"
          role="img"
          aria-label="Pipeline definition has changed since this task was created"
          :title="pipelineStaleTitle"
        >⚠</span>
      </dd>
    </div>
    <div>
      <dt>State</dt>
      <dd>
        <!-- audit-ok: dynamic-binding (badge bg by stage family) -->
        <span
          class="inline-flex items-center max-w-full px-[0.55rem] py-1 rounded-[4px] text-[var(--color-text-primary)] font-mono text-[length:var(--text-2xs)] font-[var(--weight-title)] leading-[1.4]"
          :style="{ background: badgeBg(stateTone) }"
        >
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
    <div class="col-[1/-1]">
      <dt>File</dt>
      <dd :title="fileLabel">{{ fileLabel }}</dd>
    </div>
  </dl>
</template>
