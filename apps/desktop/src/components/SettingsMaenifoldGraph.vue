<script setup lang="ts">
import type { MaenifoldSettings } from '../lib/types';

defineProps<{
  form: MaenifoldSettings;
  onCommunityDefaultToggle: (event: Event) => void;
  onUpdateCommunityRecompute: (event: Event) => void;
}>();
</script>

<template>
  <div class="settings-groups" role="group" aria-label="Maenifold graph settings">
    <details class="settings-card details-section" open>
      <summary><h4>Decay</h4></summary>
      <div class="section-grid">
        <label class="form-group">
          <span>Grace days sequential</span>
          <input v-model.number="form.decay.graceDaysSequential" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Grace days workflows</span>
          <input v-model.number="form.decay.graceDaysWorkflows" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Grace days default</span>
          <input v-model.number="form.decay.graceDaysDefault" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Half life days</span>
          <input v-model.number="form.decay.halfLifeDays" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Decay function</span>
          <select v-model="form.decay.function">
            <option value="power-law">power-law</option>
            <option value="exponential">exponential</option>
            <option value="linear">linear</option>
          </select>
        </label>
      </div>
    </details>

    <details class="settings-card details-section" open>
      <summary><h4>Community Detection</h4></summary>
      <div class="section-grid">
        <label class="form-group">
          <span>Louvain gamma</span>
          <input v-model.number="form.community.louvainGamma" type="number" min="0" step="0.1" />
        </label>
        <label class="form-group">
          <span>Min shared neighbors</span>
          <input v-model.number="form.community.minSharedNeighbors" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Min overlap</span>
          <input v-model.number="form.community.minOverlap" type="number" min="0" max="1" step="0.05" />
        </label>
        <label class="form-group">
          <span>Max siblings</span>
          <input v-model.number="form.community.maxSiblings" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Debounce ms</span>
          <input v-model.number="form.community.debounceMs" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>NMI threshold</span>
          <input v-model.number="form.community.nmiThreshold" type="number" min="0" max="1" step="0.05" />
        </label>
        <div class="optional-field">
          <label class="form-group">
            <span>Recompute file threshold</span>
            <input
              :disabled="form.community.recomputeFileThreshold === null"
              :value="form.community.recomputeFileThreshold ?? 0"
              type="number"
              min="0"
              step="1"
              @input="onUpdateCommunityRecompute"
            />
          </label>
          <label class="checkbox-inline">
            <input
              :checked="form.community.recomputeFileThreshold === null"
              type="checkbox"
              @change="onCommunityDefaultToggle"
            />
            <span>Use default</span>
          </label>
        </div>
      </div>
    </details>

    <details class="settings-card details-section" open>
      <summary><h4>Vector / Embedding</h4></summary>
      <div class="section-grid">
        <label class="checkbox-group"><input v-model="form.vector.embeddingLogs" type="checkbox" /><span>Embedding logs</span></label>
        <label class="checkbox-group"><input v-model="form.vector.vectorLogs" type="checkbox" /><span>Vector logs</span></label>
        <label class="checkbox-group"><input v-model="form.vector.disableVector" type="checkbox" /><span>Disable vector</span></label>
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

.form-group input,
.form-group select {
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
</style>
