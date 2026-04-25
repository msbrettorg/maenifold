<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, removeStorageValue, writeStorageValue } from '../lib/local-storage';
import { initTelemetry, setTelemetryEnabled } from '../lib/telemetry';
import { type FontDensity, usePreferencesStore } from '../stores/preferences';
import { useVoiceHandyStore } from '@/stores/voice-handy';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';

const prefsStore = usePreferencesStore();
const voiceStore = useVoiceHandyStore();
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

const telemetryEnabled = ref(false);
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
    telemetryEnabled.value = readStorageValue<boolean>(PREFS_STORE_PATH, 'telemetryEnabled') ?? false;
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
    <Card>
      <CardHeader class="flex flex-row items-start justify-between gap-4 max-sm:flex-col max-sm:items-stretch">
        <div class="flex flex-col gap-1">
          <CardTitle>Preferences</CardTitle>
          <CardDescription>Persisted locally on this device.</CardDescription>
        </div>
        <span
          class="self-start rounded-md px-2.5 py-1 text-xs font-medium"
          :class="saving ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'"
        >
          {{ saving ? 'Saving…' : 'Local only' }}
        </span>
      </CardHeader>

      <CardContent class="flex flex-col gap-4">
        <p v-if="error" class="m-0 text-sm text-destructive">{{ error }}</p>
        <p v-else-if="status" class="m-0 text-sm text-primary">{{ status }}</p>

        <div v-if="loading" class="rounded-md border bg-muted/40 p-4 text-sm text-muted-foreground">
          Loading saved preferences…
        </div>

        <template v-else>
          <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
            <div class="flex flex-col gap-1">
              <Label for="font-density-select" class="text-sm font-semibold">Text size</Label>
              <p class="m-0 text-sm text-muted-foreground">
                Adjust the UI font scale across the entire application.
              </p>
            </div>
            <Select
              :model-value="fontDensity"
              @update:model-value="(val) => handleDensityChange(val as FontDensity)"
            >
              <SelectTrigger id="font-density-select" :disabled="saving" class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="opt in densityOptions" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="flex items-center justify-between gap-4 rounded-md border bg-muted/40 p-4 max-sm:flex-col max-sm:items-stretch">
            <div class="flex flex-col gap-1">
              <Label for="telemetry-switch" class="text-sm font-semibold">Telemetry</Label>
              <p class="m-0 text-sm text-muted-foreground">
                Off by default. Turn on to allow Application Insights telemetry on launch.
              </p>
            </div>
            <Switch
              id="telemetry-switch"
              :model-value="telemetryEnabled"
              :disabled="saving"
              @update:model-value="(val: boolean) => { telemetryEnabled = val; handleTelemetryToggle(); }"
            />
          </div>

          <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
            <div class="flex items-start justify-between gap-4 max-sm:flex-col max-sm:items-stretch">
              <div class="flex flex-col gap-1">
                <span class="text-sm font-semibold">Last working directory</span>
                <p class="m-0 text-sm text-muted-foreground">
                  The app restores this folder on the next launch when available.
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                :disabled="saving || !lastCwd"
                @click="clearLastCwd"
              >
                Clear saved folder
              </Button>
            </div>
            <code class="block break-all rounded bg-background px-2 py-1 font-mono text-xs text-muted-foreground">
              {{ displayCwd }}
            </code>
          </div>

          <!-- Voice input section -->
          <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
            <div class="flex flex-col gap-1">
              <h3 class="text-base font-semibold">Voice input</h3>
              <p class="m-0 text-sm text-muted-foreground">Speech-to-text via Handy companion app</p>
            </div>

            <div class="flex items-center gap-3">
              <Checkbox
                :checked="voiceStore.enabled"
                @update:checked="voiceStore.setEnabled($event)"
              />
              <Label>Enable voice input button</Label>
            </div>

            <div class="flex items-center gap-2 text-sm">
              <span v-if="voiceStore.installed" class="text-success">✓ Handy installed</span>
              <span v-else-if="voiceStore.installed === false" class="text-muted-foreground">
                ✗ Handy not installed — {{ voiceStore.installHint }}
              </span>
              <span v-else class="text-muted-foreground">Checking...</span>
            </div>

            <span v-if="voiceStore.lastError" class="text-danger">
              ⚠ {{ voiceStore.lastError }}
            </span>

            <Button
              v-if="voiceStore.installed"
              variant="outline"
              size="sm"
              class="self-start"
              @click="voiceStore.launch()"
            >
              {{ voiceStore.running ? 'Open Handy' : 'Launch Handy' }}
            </Button>
          </div>
        </template>
      </CardContent>
    </Card>
  </section>
</template>
