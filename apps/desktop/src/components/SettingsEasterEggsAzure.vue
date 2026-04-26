<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { WallpaperAzureSettings } from '../lib/types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

type AzureField = 'endpointEnvVar' | 'apiKeyEnvVar' | 'imageModel';

const props = defineProps<{
  azure: WallpaperAzureSettings;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (event: 'update-field', field: AzureField, value: string): void;
}>();

const fields: { key: AzureField; label: string; description: string }[] = [
  {
    key: 'endpointEnvVar',
    label: 'Endpoint env var',
    description: 'Environment variable containing the Azure OpenAI endpoint.',
  },
  {
    key: 'apiKeyEnvVar',
    label: 'API key env var',
    description: 'Environment variable containing the Azure OpenAI API key.',
  },
  {
    key: 'imageModel',
    label: 'Image model deployment',
    description: 'Deployment name used for GPT Image generation.',
  },
];

const presence = ref<Record<string, boolean>>({});
const isChecking = ref(false);
const status = ref('');

const namesToCheck = computed(() =>
  fields
    .map((field) => props.azure[field.key].trim())
    .filter((name) => name.length > 0),
);

function updateField(field: AzureField, value: string | number) {
  emit('update-field', field, String(value));
}

function fieldPresence(field: AzureField): boolean | null {
  const name = props.azure[field].trim();
  return name ? presence.value[name] ?? false : null;
}

async function refreshPresence() {
  status.value = '';

  if (!window.electronAPI?.checkEnvVarPresence) {
    presence.value = {};
    status.value = 'Environment checks are available in the desktop shell.';
    return;
  }

  const names = namesToCheck.value;
  if (names.length === 0) {
    presence.value = {};
    return;
  }

  isChecking.value = true;
  try {
    presence.value = await window.electronAPI.checkEnvVarPresence(names);
  } catch (error) {
    status.value = error instanceof Error ? error.message : String(error);
  } finally {
    isChecking.value = false;
  }
}

watch(namesToCheck, () => {
  void refreshPresence();
}, { immediate: true });
</script>

<template>
  <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
    <div class="flex items-start justify-between gap-4 max-sm:flex-col max-sm:items-stretch">
      <div class="flex flex-col gap-1">
        <h3 class="text-base font-semibold">Azure configuration</h3>
        <p class="m-0 text-sm text-muted-foreground">
          Store secret values in the environment. Maenifold only checks whether each configured name is present.
        </p>
      </div>
      <Button type="button" variant="outline" size="sm" :disabled="disabled || isChecking" @click="refreshPresence">
        {{ isChecking ? 'Checking…' : 'Recheck' }}
      </Button>
    </div>

    <p v-if="status" class="m-0 text-sm text-muted-foreground">{{ status }}</p>

    <div v-for="field in fields" :key="field.key" class="flex flex-col gap-2">
      <div class="flex items-center gap-2">
        <span
          class="size-2 rounded-full"
          :class="fieldPresence(field.key) ? 'bg-success' : 'bg-destructive'"
          :title="fieldPresence(field.key) ? 'Present in environment' : 'Not present in environment'"
          aria-hidden="true"
        />
        <Label :for="`easter-eggs-${field.key}`" class="text-sm font-semibold">
          {{ field.label }}
        </Label>
      </div>
      <Input
        :id="`easter-eggs-${field.key}`"
        :model-value="azure[field.key]"
        :disabled="disabled"
        @update:model-value="(value) => updateField(field.key, value)"
      />
      <p class="m-0 text-xs text-muted-foreground">{{ field.description }}</p>
    </div>
  </div>
</template>
