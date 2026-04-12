<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import type { MaenifoldSettings } from '../lib/types';
import type { AgentsConfig } from '../lib/types';
import { useConfigStore } from '../stores/config';

const DEFAULT_SETTINGS: MaenifoldSettings = {
  logRotation: 'daily',
  debounceMs: 150,
  autoSync: true,
  syncLogging: true,
  sessionCleanup: true,
  sqliteBusyTimeoutMs: 5000,
  syncLeaseMs: 10000,
  watcherBufferSize: 65536,
  incrementalOptimizeEvery: 40,
  incrementalVacuumMinutes: 720,
  sessionAbandonMinutes: null,
  snippetLength: null,
  syncRole: null,
  decay: {
    graceDaysSequential: 7,
    graceDaysWorkflows: 14,
    graceDaysDefault: 28,
    halfLifeDays: 30,
    function: 'power-law',
  },
  community: {
    louvainGamma: 1.0,
    minSharedNeighbors: 3,
    minOverlap: 0.4,
    maxSiblings: 10,
    debounceMs: 2000,
    nmiThreshold: 0.8,
    recomputeFileThreshold: null,
  },
  vector: {
    embeddingLogs: false,
    vectorLogs: false,
    disableVector: false,
  },
  debug: {
    trace: false,
    pipelineDriver: null,
  },
};

type OptionalNumberField = 'sessionAbandonMinutes' | 'snippetLength';

const configStore = useConfigStore();
const form = reactive<MaenifoldSettings>(cloneSettings(DEFAULT_SETTINGS));
const isLoading = ref(true);
const isSubmitting = ref(false);
const feedback = ref<{ type: 'success' | 'error'; message: string } | null>(null);

function cloneSettings(settings: MaenifoldSettings): MaenifoldSettings {
  return JSON.parse(JSON.stringify(settings)) as MaenifoldSettings;
}

function applySettings(settings: MaenifoldSettings) {
  Object.assign(form, cloneSettings(settings));
}

function setOptionalNumberEnabled(field: OptionalNumberField, enabled: boolean, defaultValue = 0) {
  form[field] = enabled ? (form[field] ?? defaultValue) : null;
}

function togglePipelineDriver(enabled: boolean) {
  form.debug.pipelineDriver = enabled ? (form.debug.pipelineDriver ?? '') : null;
}

function updateOptionalNumber(field: OptionalNumberField, event: Event) {
  form[field] = Number((event.target as HTMLInputElement).value);
}

function setCommunityRecomputeEnabled(enabled: boolean) {
  form.community.recomputeFileThreshold = enabled ? (form.community.recomputeFileThreshold ?? 0) : null;
}

function updateCommunityRecompute(event: Event) {
  form.community.recomputeFileThreshold = Number((event.target as HTMLInputElement).value);
}

function updatePipelineDriver(event: Event) {
  form.debug.pipelineDriver = (event.target as HTMLInputElement).value;
}

function updateSyncRole(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  form.syncRole = value === '__auto__' ? null : value;
}

function handleUseDefaultToggle(
  field: OptionalNumberField,
  event: Event,
  defaultValue = 0
) {
  setOptionalNumberEnabled(field, !(event.target as HTMLInputElement).checked, defaultValue);
}

function handleCommunityDefaultToggle(event: Event) {
  setCommunityRecomputeEnabled(!(event.target as HTMLInputElement).checked);
}

function handlePipelineDriverDefaultToggle(event: Event) {
  togglePipelineDriver(!(event.target as HTMLInputElement).checked);
}

async function loadSettings() {
  isLoading.value = true;
  feedback.value = null;

  try {
    const response = await fetch(`${DAEMON_BASE_URL}/api/config`);
    if (!response.ok) {
      throw new Error(`Failed to load maenifold settings: HTTP ${response.status}`);
    }

    const config = await response.json() as AgentsConfig;
    applySettings(config.settings ?? DEFAULT_SETTINGS);
  } catch (error) {
    feedback.value = {
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    isLoading.value = false;
  }
}

async function saveSettings(settings: MaenifoldSettings, successMessage: string) {
  isSubmitting.value = true;
  feedback.value = null;

  try {
    const response = await fetch(`${DAEMON_BASE_URL}/api/config/settings`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ settings: cloneSettings(settings) }),
    });
    if (!response.ok) {
      throw new Error(`Failed to save maenifold settings: HTTP ${response.status}`);
    }

    const nextConfig = await response.json() as AgentsConfig;
    configStore.updateFromEvent(nextConfig);
    applySettings(nextConfig.settings ?? settings);
    feedback.value = { type: 'success', message: successMessage };
  } catch (error) {
    feedback.value = {
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    isSubmitting.value = false;
  }
}

async function handleSave() {
  await saveSettings(form, 'Maenifold settings saved.');
}

async function handleReset() {
  await saveSettings(DEFAULT_SETTINGS, 'Maenifold settings reset to defaults.');
}

onMounted(loadSettings);
</script>

<template>
  <section class="settings-section">
    <div class="section-header">
      <div>
        <h3>Maenifold settings</h3>
        <p>Configure runtime behavior, persistence, graph tuning, and debug controls.</p>
      </div>
    </div>

    <div v-if="isLoading" class="settings-card muted">Loading maenifold settings…</div>

    <template v-else>
      <p v-if="feedback" class="feedback" :class="feedback.type">{{ feedback.message }}</p>

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
          <label class="checkbox-group">
            <input v-model="form.autoSync" type="checkbox" />
            <span>Auto sync</span>
          </label>
          <label class="checkbox-group">
            <input v-model="form.syncLogging" type="checkbox" />
            <span>Sync logging</span>
          </label>
          <label class="checkbox-group">
            <input v-model="form.sessionCleanup" type="checkbox" />
            <span>Session cleanup</span>
          </label>
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
                @input="updateCommunityRecompute"
              />
            </label>
            <label class="checkbox-inline">
              <input
                :checked="form.community.recomputeFileThreshold === null"
                type="checkbox"
                @change="handleCommunityDefaultToggle"
              />
              <span>Use default</span>
            </label>
          </div>
        </div>
      </details>

      <details class="settings-card details-section" open>
        <summary><h4>Vector / Embedding</h4></summary>
        <div class="section-grid">
          <label class="checkbox-group">
            <input v-model="form.vector.embeddingLogs" type="checkbox" />
            <span>Embedding logs</span>
          </label>
          <label class="checkbox-group">
            <input v-model="form.vector.vectorLogs" type="checkbox" />
            <span>Vector logs</span>
          </label>
          <label class="checkbox-group">
            <input v-model="form.vector.disableVector" type="checkbox" />
            <span>Disable vector</span>
          </label>
        </div>
      </details>

      <details class="settings-card details-section" open>
        <summary><h4>Debug</h4></summary>
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
                @input="updatePipelineDriver"
              />
            </label>
            <label class="checkbox-inline">
              <input
                :checked="form.debug.pipelineDriver === null"
                type="checkbox"
                @change="handlePipelineDriverDefaultToggle"
              />
              <span>Use default</span>
            </label>
          </div>
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
                @input="updateOptionalNumber('sessionAbandonMinutes', $event)"
              />
            </label>
            <label class="checkbox-inline">
              <input
                :checked="form.sessionAbandonMinutes === null"
                type="checkbox"
                @change="handleUseDefaultToggle('sessionAbandonMinutes', $event)"
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
                @input="updateOptionalNumber('snippetLength', $event)"
              />
            </label>
            <label class="checkbox-inline">
              <input
                :checked="form.snippetLength === null"
                type="checkbox"
                @change="handleUseDefaultToggle('snippetLength', $event)"
              />
              <span>Use default</span>
            </label>
          </div>
          <label class="form-group">
            <span>Sync role</span>
            <select :value="form.syncRole ?? '__auto__'" @change="updateSyncRole">
              <option value="__auto__">auto</option>
              <option value="primary">primary</option>
              <option value="replica">replica</option>
            </select>
          </label>
        </div>
      </details>

      <div class="form-actions">
        <button class="save-btn" type="button" :disabled="isSubmitting" @click="handleSave">
          {{ isSubmitting ? 'Saving…' : 'Save' }}
        </button>
        <button class="reset-btn" type="button" :disabled="isSubmitting" @click="handleReset">
          Reset to Defaults
        </button>
      </div>
    </template>
  </section>
</template>

<style scoped>
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.section-header h3,
.section-header p,
.details-section summary h4,
.feedback {
  margin: 0;
}

.section-header p,
.muted {
  color: var(--text-secondary);
}

.settings-card {
  padding: 1rem;
  border-radius: 8px;
  background: var(--bg-panel);
}

.details-section {
  border: 1px solid var(--border-default);
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

.form-group span,
.checkbox-group span,
.checkbox-inline span {
  color: var(--text-primary);
}

.form-group input,
.form-group select {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
}

.checkbox-group,
.checkbox-inline,
.form-actions {
  display: flex;
  gap: 0.65rem;
}

.checkbox-group {
  align-items: center;
  min-height: 42px;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 8px;
  background: var(--bg-main);
}

.checkbox-inline {
  align-items: center;
}

.optional-field {
  padding: 0.9rem;
  border: 1px solid var(--border-default);
  border-radius: 8px;
  background: var(--bg-main);
}

.optional-field-full {
  grid-column: 1 / -1;
}

.feedback {
  padding: 0.75rem 0.9rem;
  border-radius: 8px;
}

.feedback.success {
  background: color-mix(in srgb, var(--border-focus) 18%, transparent);
  color: var(--text-primary);
}

.feedback.error {
  background: color-mix(in srgb, var(--border-error) 18%, transparent);
  color: var(--border-error);
}

.save-btn,
.reset-btn {
  padding: 0.55rem 1rem;
  border-radius: 6px;
  cursor: pointer;
}

.save-btn {
  border: none;
  background: var(--border-focus);
  color: white;
}

.reset-btn {
  border: 1px solid var(--border-error);
  background: transparent;
  color: var(--border-error);
}

@media (max-width: 640px) {
  .form-actions {
    flex-direction: column;
  }
}
</style>
