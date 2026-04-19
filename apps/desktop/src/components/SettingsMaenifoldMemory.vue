<script setup lang="ts">
import type { MaenifoldSettings } from '../lib/types';

type OptionalNumberField = 'sessionAbandonMinutes' | 'snippetLength';

defineProps<{
  form: MaenifoldSettings;
  onUpdateOptionalNumber: (field: OptionalNumberField, event: Event) => void;
  onUseDefaultToggle: (field: OptionalNumberField, event: Event, defaultValue?: number) => void;
  onUpdateSyncRole: (event: Event) => void;
}>();
</script>

<template>
  <div class="settings-groups" role="group" aria-label="Maenifold memory settings">
    <details class="settings-card details-section" open>
      <summary><h4>General</h4></summary>
      <div class="section-grid">
        <label class="form-group">
          <span>Log rotation</span>
          <select v-model="form.logRotation">
            <option value="daily">daily</option>
            <option value="hourly">hourly</option>
            <option value="never">never</option>
          </select>
        </label>
        <label class="form-group">
          <span>Debounce ms</span>
          <input v-model.number="form.debounceMs" type="number" min="0" step="1" />
        </label>
        <label class="checkbox-group"><input v-model="form.autoSync" type="checkbox" /><span>Auto sync</span></label>
        <label class="checkbox-group"><input v-model="form.syncLogging" type="checkbox" /><span>Sync logging</span></label>
        <label class="checkbox-group"><input v-model="form.sessionCleanup" type="checkbox" /><span>Session cleanup</span></label>
      </div>
    </details>

    <details class="settings-card details-section" open>
      <summary><h4>Database</h4></summary>
      <div class="section-grid">
        <label class="form-group">
          <span>SQLite busy timeout ms</span>
          <input v-model.number="form.sqliteBusyTimeoutMs" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Sync lease ms</span>
          <input v-model.number="form.syncLeaseMs" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Watcher buffer size</span>
          <input v-model.number="form.watcherBufferSize" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Incremental optimize every</span>
          <input v-model.number="form.incrementalOptimizeEvery" type="number" min="0" step="1" />
        </label>
        <label class="form-group">
          <span>Incremental vacuum minutes</span>
          <input v-model.number="form.incrementalVacuumMinutes" type="number" min="0" step="1" />
        </label>
      </div>
    </details>

    <details class="settings-card details-section" open>
      <summary><h4>Session</h4></summary>
      <div class="section-grid">
        <div class="optional-field">
          <label class="form-group">
            <span>Session abandon minutes</span>
            <input
              :disabled="form.sessionAbandonMinutes === null"
              :value="form.sessionAbandonMinutes ?? 0"
              type="number"
              min="0"
              step="1"
              @input="onUpdateOptionalNumber('sessionAbandonMinutes', $event)"
            />
          </label>
          <label class="checkbox-inline">
            <input
              :checked="form.sessionAbandonMinutes === null"
              type="checkbox"
              @change="onUseDefaultToggle('sessionAbandonMinutes', $event)"
            />
            <span>Use default</span>
          </label>
        </div>

        <div class="optional-field">
          <label class="form-group">
            <span>Snippet length</span>
            <input
              :disabled="form.snippetLength === null"
              :value="form.snippetLength ?? 0"
              type="number"
              min="0"
              step="1"
              @input="onUpdateOptionalNumber('snippetLength', $event)"
            />
          </label>
          <label class="checkbox-inline">
            <input :checked="form.snippetLength === null" type="checkbox" @change="onUseDefaultToggle('snippetLength', $event)" />
            <span>Use default</span>
          </label>
        </div>

        <label class="form-group">
          <span>Sync role</span>
          <select :value="form.syncRole ?? '__auto__'" @change="onUpdateSyncRole">
            <option value="__auto__">auto</option>
            <option value="primary">primary</option>
            <option value="replica">replica</option>
          </select>
        </label>
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
  background: var(--color-bg-elevated);
}

.details-section {
  border: 1px solid var(--color-border-subtle);
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
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
  color: var(--color-text-primary);
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
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
}

.optional-field {
  padding: 0.9rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
}
</style>
