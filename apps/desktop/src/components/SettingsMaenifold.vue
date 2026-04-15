<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import type { AgentsConfig, MaenifoldSettings } from '../lib/types';
import { useConfigStore } from '../stores/config';
import SettingsMaenifoldGraph from './SettingsMaenifoldGraph.vue';
import SettingsMaenifoldMemory from './SettingsMaenifoldMemory.vue';
import SettingsMaenifoldWorkflows from './SettingsMaenifoldWorkflows.vue';

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

function handleUseDefaultToggle(field: OptionalNumberField, event: Event, defaultValue = 0) {
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

    const config = (await response.json()) as AgentsConfig;
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

    const nextConfig = (await response.json()) as AgentsConfig;
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
  <section class="settings-section" role="form" aria-label="Maenifold settings">
    <div class="section-header">
      <div>
        <h3>Maenifold settings</h3>
        <p>Configure runtime behavior, persistence, graph tuning, and debug controls.</p>
      </div>
    </div>

    <div v-if="isLoading" class="settings-card muted">Loading maenifold settings…</div>

    <template v-else>
      <p v-if="feedback" class="feedback" :class="feedback.type">{{ feedback.message }}</p>

      <SettingsMaenifoldMemory
        :form="form"
        :on-update-optional-number="updateOptionalNumber"
        :on-update-sync-role="updateSyncRole"
        :on-use-default-toggle="handleUseDefaultToggle"
      />

      <SettingsMaenifoldGraph
        :form="form"
        :on-community-default-toggle="handleCommunityDefaultToggle"
        :on-update-community-recompute="updateCommunityRecompute"
      />

      <SettingsMaenifoldWorkflows
        :form="form"
        :on-pipeline-driver-default-toggle="handlePipelineDriverDefaultToggle"
        :on-update-pipeline-driver="updatePipelineDriver"
      />

      <div class="form-actions">
        <button class="save-btn" type="button" :disabled="isSubmitting" @click="handleSave">
          {{ isSubmitting ? 'Saving…' : 'Save' }}
        </button>
        <button class="reset-btn" type="button" :disabled="isSubmitting" @click="handleReset">
          Reset to defaults
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
.feedback {
  margin: 0;
}

.section-header p,
.muted {
  color: var(--muted-foreground);
}

.settings-card {
  padding: 1rem;
  border-radius: var(--radius);
  background: var(--card);
}

.feedback {
  padding: 0.75rem 0.9rem;
  border-radius: var(--radius);
}

.feedback.success {
  background: color-mix(in srgb, var(--ring) 18%, transparent);
  color: var(--foreground);
}

.feedback.error {
  background: color-mix(in srgb, var(--destructive) 18%, transparent);
  color: var(--destructive);
}

.form-actions {
  display: flex;
  gap: 0.65rem;
}

.save-btn,
.reset-btn {
  padding: 0.55rem 1rem;
  border-radius: var(--radius);
  cursor: pointer;
}

.save-btn {
  border: none;
  background: var(--ring);
  color: var(--foreground);
}

.reset-btn {
  border: 1px solid var(--destructive);
  background: transparent;
  color: var(--destructive);
}

@media (max-width: 640px) {
  .form-actions {
    flex-direction: column;
  }
}
</style>
