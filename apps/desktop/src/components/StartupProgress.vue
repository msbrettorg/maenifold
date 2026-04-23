<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';

const props = defineProps<{
  agentName: string;
  phase: string;
  logs: string[];
  elapsedSeconds: number;
  showDetails: boolean;
}>();

const emit = defineEmits<{
  cancel: [];
  toggleDetails: [];
}>();

const phaseIcon = computed(() => {
  switch (props.phase) {
    case 'starting': return '🚀';
    case 'downloading': return '📥';
    case 'installing': return '📦';
    case 'building': return '🔨';
    case 'initializing': return '⚙️';
    case 'connecting': return '🔗';
    case 'ready': return '✅';
    case 'agent_closed':
    case 'error': return '❌';
    default: return '⏳';
  }
});

type StartupStatus = 'spawning' | 'initializing' | 'ready' | 'error';

const status = computed<StartupStatus>(() => {
  if (props.phase === 'agent_closed' || props.phase === 'error') return 'error';
  if (props.phase === 'ready') return 'ready';
  if (props.phase === 'initializing') return 'initializing';
  return 'spawning';
});

const phaseText = computed(() => {
  switch (props.phase) {
    case 'starting': return 'Starting process...';
    case 'downloading': return 'Downloading packages...';
    case 'installing': return 'Installing dependencies...';
    case 'building': return 'Building...';
    case 'initializing': return 'Initializing agent...';
    case 'connecting': return 'Connecting to agent...';
    case 'ready': return 'Agent ready.';
    case 'agent_closed':
    case 'error': return 'Startup failed. Agent closed unexpectedly.';
    default: return 'Please wait...';
  }
});

const formattedTime = computed(() => {
  const mins = Math.floor(props.elapsedSeconds / 60);
  const secs = props.elapsedSeconds % 60;
  if (mins > 0) {
    return `${mins}m ${secs}s`;
  }
  return `${secs}s`;
});

const isLongWait = computed(() => props.elapsedSeconds > 10);
const progressValue = computed(() => {
  switch (props.phase) {
    case 'starting': return 10;
    case 'downloading': return 25;
    case 'installing': return 50;
    case 'building': return 70;
    case 'initializing': return 85;
    case 'connecting': return 95;
    default: return 0;
  }
});

const phasePillBgClass = computed(() => {
  switch (status.value) {
    case 'initializing': return 'bg-warning';
    case 'ready': return 'bg-success';
    case 'error': return 'bg-danger';
    default: return 'bg-[var(--accent-prompt)]';
  }
});
</script>

<template>
  <section class="p-4 bg-surface-container rounded-sm mt-3">
    <div class="flex justify-between items-center gap-2 mb-3">
      <span class="font-[var(--weight-title)] text-text-text-primary">Connecting to {{ agentName }}...</span>
      <div class="inline-flex items-center gap-2">
        <span
          :class="[
            'inline-flex items-center py-0.5 px-2 rounded-xl text-tiny text-[var(--text)]',
            phasePillBgClass,
          ]"
        >{{ status }}</span>
        <span class="text-[length:var(--text-caption)] text-text-text-secondary font-mono">{{ formattedTime }}</span>
      </div>
    </div>

    <!-- audit-ok: no-raw-progress -->
    <div
      class="flex items-center gap-2 p-2 bg-surface-high rounded-sm mb-3"
      role="progressbar"
      aria-label="Startup progress"
      :aria-valuemin="0"
      :aria-valuemax="100"
      :aria-valuenow="progressValue"
      :aria-valuetext="phaseText"
    >
      <span class="text-[length:var(--text-title)]">{{ phaseIcon }}</span>
      <span class="text-text-text-secondary text-[length:var(--text-body-sm)]">{{ phaseText }}</span>
    </div>

    <!-- audit-ok: dynamic-binding inline color-mix() blends bg-warning into surface-container for first-run hint background. -->
    <div
      v-if="isLongWait && !showDetails"
      class="text-[length:var(--text-caption)] text-text-text-secondary italic mb-3 p-2 rounded-sm"
      style="background: color-mix(in srgb, var(--bg-warning) 18%, var(--surface-container));"
    >
      First run may take longer while dependencies are installed.
    </div>

    <div class="flex gap-2 justify-between">
      <Button
        variant="ghost"
        size="sm"
        class="py-1.5 px-3 rounded-sm bg-surface-high text-text-text-secondary text-[length:var(--text-caption)] hover:bg-surface-highest"
        @click="emit('toggleDetails')"
      >
        {{ showDetails ? 'Hide details ▲' : 'Show details ▼' }}
      </Button>
      <!-- audit-ok: dynamic-binding hover blends danger color into surface-high; cannot tokenize a hover-only blend without a custom variant. -->
      <Button
        variant="ghost"
        size="sm"
        class="py-1.5 px-3 rounded-sm bg-surface-high text-text-text-secondary text-[length:var(--text-caption)] hover:bg-[color-mix(in_srgb,var(--color-danger)_18%,var(--surface-high))] hover:text-danger"
        @click="emit('cancel')"
      >
        Cancel
      </Button>
    </div>

    <div v-if="showDetails" class="mt-3 rounded-sm overflow-hidden">
      <div class="py-1.5 px-2 bg-surface-high text-tiny font-[var(--weight-title)] text-text-text-secondary">Output</div>
      <div class="logs-content max-h-[150px] overflow-y-auto p-2 bg-surface-high font-mono text-tiny">
        <div
          v-for="(log, index) in logs.slice(-50)"
          :key="index"
          class="text-text-text-secondary whitespace-pre-wrap break-all leading-snug"
        >
          {{ log }}
        </div>
        <div v-if="logs.length === 0" class="text-text-text-secondary italic">
          Waiting for output...
        </div>
      </div>
    </div>
  </section>
</template>

<!-- audit-ok: max-style-rules ::-webkit-scrollbar pseudo-elements are not addressable via Tailwind utilities; scoped to .logs-content for the agent output panel. -->
<style scoped>
.logs-content {
  scrollbar-width: thin;
  scrollbar-color: var(--color-border-subtle) transparent;
}
.logs-content::-webkit-scrollbar { width: 8px; }
.logs-content::-webkit-scrollbar-track { background: transparent; }
.logs-content::-webkit-scrollbar-thumb {
  background: var(--color-border-subtle);
  border-radius: 4px;
}
.logs-content::-webkit-scrollbar-thumb:hover { background: var(--color-text-secondary); }
</style>
