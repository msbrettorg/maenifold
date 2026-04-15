<script setup lang="ts">
import type { MaenifoldSettings } from '../lib/types';

defineProps<{
  form: MaenifoldSettings;
  onUpdatePipelineDriver: (event: Event) => void;
  onPipelineDriverDefaultToggle: (event: Event) => void;
}>();
</script>

<template>
  <div class="settings-groups" role="group" aria-label="Maenifold workflow settings">
    <details class="settings-card details-section" open>
      <summary><h4>Workflows &amp; Debug</h4></summary>
      <div class="section-grid">
        <label class="checkbox-group">
          <input v-model="form.debug.trace" type="checkbox" />
          <span>Trace logging</span>
        </label>

        <div class="optional-field optional-field-full">
          <label class="form-group">
            <span>Pipeline driver</span>
            <input
              :disabled="form.debug.pipelineDriver === null"
              :value="form.debug.pipelineDriver ?? ''"
              type="text"
              placeholder="inline"
              @input="onUpdatePipelineDriver"
            />
          </label>
          <label class="checkbox-inline">
            <input :checked="form.debug.pipelineDriver === null" type="checkbox" @change="onPipelineDriverDefaultToggle" />
            <span>Use default</span>
          </label>
        </div>
      </div>
    </details>
  </div>
</template>

<style scoped>
.settings-groups {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.settings-card {
  padding: 1rem;
  border-radius: var(--radius);
  background: var(--card);
}

.details-section {
  border: 1px solid var(--border);
}

.details-section summary {
  cursor: pointer;
  list-style: none;
}

.details-section summary::-webkit-details-marker {
  display: none;
}

.details-section[open] summary {
  margin-bottom: 0.75rem;
}

.details-section summary h4 {
  margin: 0;
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.9rem;
}

.form-group,
.optional-field {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.form-group input {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--background);
  color: var(--foreground);
}

.checkbox-group,
.checkbox-inline {
  display: flex;
  gap: 0.65rem;
  align-items: center;
}

.checkbox-group {
  min-height: 42px;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--background);
}

.optional-field {
  padding: 0.9rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--background);
}

.optional-field-full {
  grid-column: 1 / -1;
}
</style>
