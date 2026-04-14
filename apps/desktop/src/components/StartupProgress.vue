<script setup lang="ts">
import { computed } from 'vue';

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
    default: return '⏳';
  }
});

const phaseText = computed(() => {
  switch (props.phase) {
    case 'starting': return 'Starting process...';
    case 'downloading': return 'Downloading packages...';
    case 'installing': return 'Installing dependencies...';
    case 'building': return 'Building...';
    case 'initializing': return 'Initializing agent...';
    case 'connecting': return 'Connecting to agent...';
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
      <span class="elapsed-time">{{ formattedTime }}</span>
    </div>
    
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
      <button 
        class="details-btn"
        @click="emit('toggleDetails')"
      >
        {{ showDetails ? 'Hide Details ▲' : 'Show Details ▼' }}
      </button>
      <button 
        class="cancel-btn"
        @click="emit('cancel')"
      >
        Cancel
      </button>
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
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-top: 0.75rem;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.agent-name {
  font-weight: var(--weight-title);
  color: var(--foreground);
}

.elapsed-time {
  font-size: var(--text-caption);
  color: var(--muted-foreground);
  font-family: var(--font-code);
}

.progress-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem;
  background: var(--muted);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.phase-icon {
  font-size: var(--text-title);
}

.phase-text {
  color: var(--muted-foreground);
  font-size: var(--text-body-sm);
}

.first-run-hint {
  font-size: var(--text-caption);
  color: var(--muted-foreground);
  font-style: italic;
  margin-bottom: 0.75rem;
  padding: 0.5rem;
  background: var(--bg-warning);
  border-radius: 4px;
}

.progress-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: space-between;
}

.details-btn {
  padding: 0.375rem 0.75rem;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: transparent;
  color: var(--muted-foreground);
  font-size: var(--text-caption);
  cursor: pointer;
}

.details-btn:hover {
  background: var(--muted);
}

.cancel-btn {
  padding: 0.375rem 0.75rem;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: transparent;
  color: var(--muted-foreground);
  font-size: var(--text-caption);
  cursor: pointer;
}

.cancel-btn:hover {
  background: var(--muted);
  border-color: var(--destructive);
  color: var(--destructive);
}

.logs-container {
  margin-top: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
}

.logs-header {
  padding: 0.375rem 0.5rem;
  background: var(--muted);
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  color: var(--muted-foreground);
  border-bottom: 1px solid var(--border);
}

.logs-content {
  max-height: 150px;
  overflow-y: auto;
  padding: 0.5rem;
  background: var(--muted);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.log-line {
  color: var(--muted-foreground);
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.4;
}

.no-logs {
  color: var(--muted-foreground);
  font-style: italic;
}
</style>
