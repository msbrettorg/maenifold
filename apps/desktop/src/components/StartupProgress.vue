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
</script>

<template>
  <section class="startup-progress">
    <div class="progress-header">
      <span class="agent-name">Connecting to {{ agentName }}...</span>
      <div class="progress-meta">
        <span class="phase-pill" :class="`phase-pill--${status}`">{{ status }}</span>
        <span class="elapsed-time">{{ formattedTime }}</span>
      </div>
    </div>
    
    <!-- audit-ok: no-raw-progress -->
    <div
      class="progress-status"
      role="progressbar"
      aria-label="Startup progress"
      :aria-valuemin="0"
      :aria-valuemax="100"
      :aria-valuenow="progressValue"
      :aria-valuetext="phaseText"
    >
      <span class="phase-icon">{{ phaseIcon }}</span>
      <span class="phase-text">{{ phaseText }}</span>
    </div>
    
    <div v-if="isLongWait && !showDetails" class="first-run-hint">
      First run may take longer while dependencies are installed.
    </div>
    
    <div class="progress-actions">
      <Button 
        variant="ghost"
        size="sm"
        class="details-btn"
        @click="emit('toggleDetails')"
      >
        {{ showDetails ? 'Hide details ▲' : 'Show details ▼' }}
      </Button>
      <Button 
        variant="ghost"
        size="sm"
        class="cancel-btn"
        @click="emit('cancel')"
      >
        Cancel
      </Button>
    </div>
    
    <div v-if="showDetails" class="logs-container">
      <div class="logs-header">Output</div>
      <div class="logs-content">
        <div 
          v-for="(log, index) in logs.slice(-50)" 
          :key="index"
          class="log-line"
        >
          {{ log }}
        </div>
        <div v-if="logs.length === 0" class="no-logs">
          Waiting for output...
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.startup-progress {
  padding: 1rem;
  background: var(--surface-container);
  border-radius: 4px;
  margin-top: 0.75rem;
}
.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}
.agent-name { font-weight: var(--weight-title); color: var(--color-text-primary); }
.progress-meta { display: inline-flex; align-items: center; gap: 0.5rem; }
.phase-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  font-size: var(--text-tiny);
  color: var(--text);
  background: var(--accent-prompt);
}
.phase-pill--initializing { background: var(--amber); }
.phase-pill--ready { background: var(--sage); }
.phase-pill--error { background: var(--terracotta); }
.elapsed-time {
  font-size: var(--text-caption);
  color: var(--color-text-secondary);
  font-family: var(--font-code);
}
.progress-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem;
  background: var(--surface-high);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}
.phase-icon { font-size: var(--text-title); }
.phase-text { color: var(--color-text-secondary); font-size: var(--text-body-sm); }
.first-run-hint {
  font-size: var(--text-caption);
  color: var(--color-text-secondary);
  font-style: italic;
  margin-bottom: 0.75rem;
  padding: 0.5rem;
  background: color-mix(in srgb, var(--bg-warning) 18%, var(--surface-container));
  border-radius: 4px;
}
.progress-actions { display: flex; gap: 0.5rem; justify-content: space-between; }
.details-btn {
  padding: 0.375rem 0.75rem;
  border: none;
  border-radius: 4px;
  background: var(--surface-high);
  color: var(--color-text-secondary);
  font-size: var(--text-caption);
  cursor: pointer;
}
.details-btn:hover { background: var(--surface-highest); }
.cancel-btn {
  padding: 0.375rem 0.75rem;
  border: none;
  border-radius: 4px;
  background: var(--surface-high);
  color: var(--color-text-secondary);
  font-size: var(--text-caption);
  cursor: pointer;
}
.cancel-btn:hover {
  background: color-mix(in srgb, var(--color-danger) 18%, var(--surface-high));
  color: var(--color-danger);
}
.logs-container {
  margin-top: 0.75rem;
  border-radius: 4px;
  overflow: hidden;
}
.logs-header {
  padding: 0.375rem 0.5rem;
  background: var(--surface-high);
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  color: var(--color-text-secondary);
}
.logs-content {
  max-height: 150px;
  overflow-y: auto;
  padding: 0.5rem;
  background: var(--surface-high);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
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
.log-line {
  color: var(--color-text-secondary);
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.4;
}
.no-logs { color: var(--color-text-secondary); font-style: italic; }
</style>
