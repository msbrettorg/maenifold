<script setup lang="ts">
defineProps<{
  hasAgents: boolean;
  canCreateSession: boolean;
}>();

const emit = defineEmits<{
  'new-session': [];
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
      <h2 class="welcome-title">maenifold</h2>
      <p class="welcome-hint">
        Select an agent and working directory in the sidebar, then start a session.
      </p>

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
  align-items: center;
  gap: 1rem;
  max-width: 28rem;
  text-align: center;
}
.welcome-title {
  margin: 0;
  font-family: var(--font-display);
  font-size: var(--text-2xl);
  font-weight: var(--weight-display);
  color: var(--foreground);
  letter-spacing: 0.04em;
}
.welcome-hint {
  margin: 0;
  color: var(--muted-foreground);
  font-size: var(--text-md);
  line-height: 1.5;
}
.starter-examples {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.5rem;
  margin-top: 0.25rem;
}
.example-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  border-radius: 4px;
  background: #2a2a2a;
  border: 1px solid rgba(66, 71, 83, 0.15);
  color: var(--muted-foreground);
  font-size: var(--text-sm);
}
.chip-icon { font-size: var(--text-sm); }
.cta-btn {
  margin-top: 0.5rem;
  padding: 0.625rem 2rem;
  border: none;
  border-radius: 4px;
  background: #4d8eff;
  color: var(--primary-foreground);
  font-size: var(--text-md);
  font-weight: var(--weight-display);
  cursor: pointer;
  transition: background 0.15s;
}
.cta-btn:hover:not(:disabled) { background: #639bff; }
.cta-btn:disabled { cursor: not-allowed; opacity: 0.5; }
.no-agents {
  margin: 0;
  color: #707070;
  font-size: var(--text-caption);
}
</style>
