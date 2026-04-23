<!-- audit-ok: max-loc -->
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, removeStorageValue, writeStorageValue } from '../lib/local-storage';
import { initTelemetry, setTelemetryEnabled } from '../lib/telemetry';
import { type FontDensity, usePreferencesStore } from '../stores/preferences';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

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

const displayCwd = computed(() =>
  lastCwd.value ? lastCwd.value.split('/').filter(Boolean).pop() ?? lastCwd.value : 'No working directory saved.',
);
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
  <section class="flex flex-col gap-4" role="form" aria-label="Preferences">
    <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
      <div class="flex justify-between gap-4 mb-3 text-[11px] font-semibold tracking-[0.06em] text-text-tertiary max-sm:flex-col max-sm:items-stretch">
        <div>
          <h3 class="m-0 font-display font-medium">Preferences</h3>
          <p class="m-0 text-text-secondary">Persisted locally on this device.</p>
        </div>
        <span class="rounded-[var(--radius)] px-[0.6rem] py-[0.35rem] bg-bg-elevated text-[0.75rem]" :class="saving ? 'text-brand' : 'text-text-secondary'">
          {{ saving ? 'Saving…' : 'Local only' }}
        </span>
      </div>

      <p v-if="error" class="m-0 text-danger">{{ error }}</p>
      <p v-else-if="status" class="m-0 text-brand">{{ status }}</p>

      <div v-if="loading" class="p-4 rounded-[var(--radius)] bg-bg-elevated">Loading saved preferences…</div>

      <template v-else>
        <div class="p-4 rounded-[var(--radius)] bg-bg-elevated">
          <div class="flex justify-between gap-4 max-sm:flex-col max-sm:items-stretch">
            <div>
              <strong>Text size</strong>
              <p class="m-0 text-text-secondary">Adjust the UI font scale across the entire application.</p>
            </div>
          </div>
          <div class="flex gap-0 mt-3 border border-border-subtle rounded-[var(--radius)] overflow-hidden">
            <Select :model-value="fontDensity" @update:model-value="(val) => handleDensityChange(val as FontDensity)">
              <SelectTrigger :disabled="saving" class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="opt in densityOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div class="p-4 rounded-[var(--radius)] bg-bg-elevated flex justify-between gap-4 items-center max-sm:flex-col max-sm:items-stretch">
          <div>
            <strong>Telemetry</strong>
            <p class="m-0 text-text-secondary">Allow the app to initialize Application Insights telemetry on launch.</p>
          </div>
          <Checkbox :checked="telemetryEnabled" :disabled="saving" @update:checked="(val: boolean) => { telemetryEnabled = val; handleTelemetryToggle() }" />
        </div>

        <div class="p-4 rounded-[var(--radius)] bg-bg-elevated">
          <div class="flex justify-between gap-4 max-sm:flex-col max-sm:items-stretch">
            <div>
              <strong>Last working directory</strong>
              <p class="m-0 text-text-secondary">The app restores this folder on the next launch when available.</p>
            </div>
            <Button variant="outline" class="px-[0.85rem] py-2 border border-border-subtle rounded-[var(--radius)] bg-transparent text-inherit cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed" :disabled="saving || !lastCwd" @click="clearLastCwd">
              Clear saved folder
            </Button>
          </div>
          <code class="block mt-3 m-0 break-all font-code text-[0.75rem] text-text-secondary">{{ displayCwd }}</code>
        </div>
      </template>
    </div>
  </section>
</template>
