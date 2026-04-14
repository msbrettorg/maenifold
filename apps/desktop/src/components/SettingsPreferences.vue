<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, removeStorageValue, writeStorageValue } from '../lib/local-storage';
import { initTelemetry, setTelemetryEnabled } from '../lib/telemetry';
import { type FontDensity, usePreferencesStore } from '../stores/preferences';

const prefsStore = usePreferencesStore();
const { fontDensity } = storeToRefs(prefsStore);

const densityOptions: { value: FontDensity; label: string }[] = [
  { value: 'compact', label: 'Compact' },
  { value: 'default', label: 'Default' },
  { value: 'comfortable', label: 'Comfortable' },
];

function handleDensityChange(density: FontDensity) {
  prefsStore.setFontDensity(density);
  status.value = 'Preferences saved.';
}

const telemetryEnabled = ref(true);
const lastCwd = ref('');
const loading = ref(true);
const saving = ref(false);
const error = ref('');
const status = ref('');

async function persistPreference(key: string, value: string | boolean) {
  saving.value = true;
  error.value = '';
  status.value = '';

  try {
    writeStorageValue(PREFS_STORE_PATH, key, value);
    status.value = 'Preferences saved.';
  } catch (persistError) {
    error.value = persistError instanceof Error ? persistError.message : String(persistError);
  } finally {
    saving.value = false;
  }
}

async function handleTelemetryToggle() {
  await persistPreference('telemetryEnabled', telemetryEnabled.value);

  if (error.value) return;

  if (telemetryEnabled.value) {
    await initTelemetry(true);
  } else {
    setTelemetryEnabled(false);
  }
}

async function clearLastCwd() {
  saving.value = true;
  error.value = '';
  status.value = '';

  try {
    removeStorageValue(PREFS_STORE_PATH, 'lastCwd');
    lastCwd.value = '';
    status.value = 'Preferences saved.';
  } catch (persistError) {
    error.value = persistError instanceof Error ? persistError.message : String(persistError);
  } finally {
    saving.value = false;
  }
}

onMounted(async () => {
  try {
    telemetryEnabled.value = readStorageValue<boolean>(PREFS_STORE_PATH, 'telemetryEnabled') ?? true;
    lastCwd.value = readStorageValue<string>(PREFS_STORE_PATH, 'lastCwd') ?? '';
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <section class="settings-section">
    <div class="section-header">
      <div>
        <h3>Preferences</h3>
        <p>These values are persisted locally under <code>{{ PREFS_STORE_PATH }}</code>.</p>
      </div>
      <span class="status-chip" :class="{ saving }">
        {{ saving ? 'Saving…' : 'Local only' }}
      </span>
    </div>

    <p v-if="error" class="message error">{{ error }}</p>
    <p v-else-if="status" class="message success">{{ status }}</p>

    <div v-if="loading" class="card">Loading saved preferences…</div>

    <template v-else>
      <div class="card">
        <div class="card-header">
          <div>
            <strong>Text size</strong>
            <p>Adjust the UI font scale across the entire application.</p>
          </div>
        </div>
        <div class="density-control">
          <label
            v-for="opt in densityOptions"
            :key="opt.value"
            class="density-option"
            :class="{ active: fontDensity === opt.value }"
          >
            <input
              type="radio"
              name="fontDensity"
              :value="opt.value"
              :checked="fontDensity === opt.value"
              :disabled="saving"
              @change="handleDensityChange(opt.value)"
            />
            {{ opt.label }}
          </label>
        </div>
      </div>

      <label class="card toggle-card">
        <div>
          <strong>Telemetry</strong>
          <p>Allow the app to initialize Application Insights telemetry on launch.</p>
        </div>
        <input v-model="telemetryEnabled" type="checkbox" :disabled="saving" @change="handleTelemetryToggle" />
      </label>

      <div class="card">
        <div class="card-header">
          <div>
            <strong>Last working directory</strong>
            <p>The app restores this folder on the next launch when available.</p>
          </div>
          <button class="secondary-btn" :disabled="saving || !lastCwd" @click="clearLastCwd">
            Clear saved folder
          </button>
        </div>
        <code class="saved-value">{{ lastCwd || 'No working directory saved.' }}</code>
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

.section-header,
.toggle-card,
.card-header {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
}

.section-header h3,
.section-header p,
.message,
.toggle-card p,
.card-header p,
.saved-value {
  margin: 0;
}

.section-header p,
.toggle-card p,
.card-header p,
.saved-value {
  color: var(--text-secondary);
}

.card,
.status-chip {
  border-radius: 8px;
}

.card {
  padding: 1rem;
  background: var(--bg-panel);
}

.toggle-card {
  align-items: center;
}

.toggle-card input {
  width: 1.1rem;
  height: 1.1rem;
}

.status-chip {
  padding: 0.35rem 0.6rem;
  background: var(--bg-panel);
  color: var(--text-secondary);
  font-size: var(--text-caption);
}

.status-chip.saving,
.message.success {
  color: var(--border-focus);
}

.message.error {
  color: var(--border-error);
}

.secondary-btn {
  padding: 0.5rem 0.85rem;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: transparent;
  color: inherit;
  cursor: pointer;
}

.secondary-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.saved-value {
  display: block;
  margin-top: 0.75rem;
  word-break: break-all;
  font-family: var(--font-code);
  font-size: var(--text-caption);
}

.density-control {
  display: flex;
  gap: 0;
  margin-top: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  overflow: hidden;
}

.density-option {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.45rem 0.5rem;
  cursor: pointer;
  font-size: var(--text-caption);
  color: var(--text-secondary);
  background: transparent;
  border-right: 1px solid var(--border-default);
  transition: background var(--transition-fast), color var(--transition-fast);
}

.density-option:last-child {
  border-right: none;
}

.density-option input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.density-option.active {
  background: var(--bg-surface);
  color: var(--text-primary);
}

.density-option:hover:not(.active) {
  background: var(--surface-high);
}

@media (max-width: 640px) {
  .section-header,
  .toggle-card,
  .card-header {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
