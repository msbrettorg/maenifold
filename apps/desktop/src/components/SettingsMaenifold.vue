<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import type { AgentsConfig, MaenifoldSettings } from '../lib/types';
import { useConfigStore } from '../stores/config';
import { Button } from '@/components/ui/button';
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
  <section class="flex flex-col gap-4" role="form" aria-label="Maenifold settings">
    <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
      <div class="mb-3 text-[11px] font-semibold tracking-[0.06em] text-text-tertiary">
        <div>
          <h3 class="m-0">Maenifold settings</h3>
          <p class="m-0 text-text-secondary">Configure runtime behavior, persistence, graph tuning, and debug controls.</p>
        </div>
      </div>

      <div v-if="isLoading" class="p-4 rounded-[var(--radius)] bg-bg-elevated text-text-secondary">Loading maenifold settings…</div>

      <template v-else>
        <p
          v-if="feedback"
          class="m-0 px-3.5 py-3 rounded-[var(--radius)]"
          :style="feedback.type === 'success'
            ? 'background: color-mix(in srgb, var(--color-brand) 18%, transparent); color: var(--color-text-primary);'
            : 'background: color-mix(in srgb, var(--color-danger) 18%, transparent); color: var(--color-danger);'"
        >{{ feedback.message }}</p>
        <!-- audit-ok: dynamic-binding -->

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

        <div class="flex gap-[0.65rem] max-sm:flex-col">
          <Button class="px-4 py-[0.55rem] rounded-[var(--radius)] cursor-pointer border-0 bg-brand text-text-primary" type="button" :disabled="isSubmitting" @click="handleSave">
            {{ isSubmitting ? 'Saving…' : 'Save' }}
          </Button>
          <Button variant="destructive" class="px-4 py-[0.55rem] rounded-[var(--radius)] cursor-pointer border border-danger bg-transparent text-danger" type="button" :disabled="isSubmitting" @click="handleReset">
            Reset to defaults
          </Button>
        </div>
      </template>
    </div>
  </section>
</template>
