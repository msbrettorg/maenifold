<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { load, type Store } from '@tauri-apps/plugin-store';
import { PREFS_STORE_PATH } from '../lib/constants';
import { initTelemetry, setTelemetryEnabled } from '../lib/telemetry';

const telemetryEnabled = ref(true);
const lastCwd = ref('');
const loading = ref(true);
const saving = ref(false);
const error = ref('');
const status = ref('');

let prefsStore: Store | null = null;

async function persistPreference(key: string, value: string | boolean) {
  if (!prefsStore) return;

  saving.value = true;
  error.value = '';
  status.value = '';

  try {
    await prefsStore.set(key, value);
    await prefsStore.save();
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
  await persistPreference('lastCwd', '');
  if (!error.value) lastCwd.value = '';
}

onMounted(async () => {
  try {
    prefsStore = await load(PREFS_STORE_PATH);
    telemetryEnabled.value = (await prefsStore.get<boolean>('telemetryEnabled')) ?? true;
    lastCwd.value = (await prefsStore.get<string>('lastCwd')) ?? '';
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
        <p>These values are persisted locally in <code>{{ PREFS_STORE_PATH }}</code>.</p>
      </div>
      <span class="status-chip" :class="{ saving }">
        {{ saving ? 'Saving…' : 'Local only' }}
      </span>
    </div>

    <p v-if="error" class="message error">{{ error }}</p>
    <p v-else-if="status" class="message success">{{ status }}</p>

    <div v-if="loading" class="card">Loading saved preferences…</div>

    <template v-else>
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
  font-size: var(--text-sm);
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
  font-family: var(--font-mono);
  font-size: var(--text-sm);
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
