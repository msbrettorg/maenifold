<script setup lang="ts">
import AgentSelector from './AgentSelector.vue';

const selectedAgent = defineModel<string>('selectedAgent', { required: true });

defineProps<{
  hasAgents: boolean;
  canCreateSession: boolean;
  selectedCwd: string;
}>();

const emit = defineEmits<{
  'new-session': [];
  'agent-select': [agentName: string];
  'select-folder': [];
}>();

const starterExamples = [
  { icon: '🔍', label: 'Research a topic' },
  { icon: '📝', label: 'Review code' },
  { icon: '📋', label: 'Plan a sprint' },
];
</script>

<template>
  <div class="welcome">
    <div class="welcome-inner">
      <h2 class="welcome-title">Start an agent session</h2>
      <p class="welcome-hint">
        Choose your agent and working directory, then launch a new session.
      </p>

      <div class="welcome-config">
        <AgentSelector
          v-model:selected="selectedAgent"
          @select="emit('agent-select', $event)"
        />

        <div class="cwd-picker">
          <label class="cwd-label">Working directory</label>
          <div class="cwd-row">
            <span
              class="cwd-value"
              :title="selectedCwd || 'Current directory'"
            >
              {{ selectedCwd ? selectedCwd.split(/[\\/]/).pop() : '.' }}
            </span>
            <button class="cwd-btn" type="button" title="Select folder" @click="emit('select-folder')">📁</button>
          </div>
        </div>
      </div>

      <div class="starter-examples" aria-label="Example tasks">
        <div v-for="example in starterExamples" :key="example.label" class="example-chip">
          <span class="chip-icon" aria-hidden="true">{{ example.icon }}</span>
          <span>{{ example.label }}</span>
        </div>
      </div>

      <button
        v-if="hasAgents"
        class="cta-btn"
        :disabled="!canCreateSession"
        @click="emit('new-session')"
      >
        New session
      </button>

      <p v-if="!hasAgents" class="no-agents">
        Configure agents in settings to begin.
      </p>
    </div>
  </div>
</template>

<style scoped>
.welcome {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}
.welcome-inner {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  width: min(34rem, 100%);
}
.welcome-title {
  margin: 0;
  font-family: var(--font-display);
  font-size: var(--text-title);
  font-weight: var(--weight-display);
  color: var(--text);
}
.welcome-hint {
  margin: 0;
  color: var(--text-dim);
  font-size: var(--text-body-sm);
  line-height: 1.5;
}
.welcome-config {
  display: grid;
  gap: 0.875rem;
  padding: 1rem;
  border-radius: 4px;
  background: var(--surface);
}
.cwd-picker {
  display: grid;
  gap: 0.375rem;
}
.cwd-label {
  font-size: var(--text-tiny);
  color: var(--text-dim);
}
.cwd-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.cwd-value {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  border-radius: 4px;
  padding: 0.5rem 0.625rem;
  background: var(--surface-container);
  color: var(--text);
  font-size: var(--text-caption);
}
.cwd-btn {
  border: none;
  border-radius: 4px;
  padding: 0.5rem 0.625rem;
  background: var(--surface-container);
  color: var(--text);
  cursor: pointer;
}
.cwd-btn:hover {
  background: var(--surface-high);
}
.starter-examples {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-start;
  gap: 0.5rem;
}
.example-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  border-radius: 4px;
  background: var(--surface-high);
  color: var(--muted-foreground);
  font-size: var(--text-sm);
}
.chip-icon { font-size: var(--text-sm); }
.cta-btn {
  margin-top: 0.25rem;
  align-self: flex-start;
  padding: 0.625rem 2rem;
  border: none;
  border-radius: 4px;
  background: linear-gradient(
    135deg,
    color-mix(in srgb, var(--blue) 65%, var(--background)) 0%,
    var(--blue) 100%
  );
  color: var(--text);
  font-size: var(--text-body-sm);
  font-weight: var(--weight-display);
  cursor: pointer;
  transition: opacity 0.15s;
}
.cta-btn:hover:not(:disabled) { opacity: 0.92; }
.cta-btn:disabled { cursor: not-allowed; opacity: 0.5; }
.no-agents {
  margin: 0;
  color: var(--text-dim);
  font-size: var(--text-caption);
}
</style>
