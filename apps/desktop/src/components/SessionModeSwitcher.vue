<script setup lang="ts">
import { computed, ref } from 'vue';
import type { AcceptableValue } from 'reka-ui';
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';
import { useSessionStore } from '../stores/session';
import type { SessionMode } from '../lib/types';
import { getOptionIcon } from './configOptionIcons';

const sessionStore = useSessionStore();
const pendingModeId = ref<string | null>(null);
const errorMessage = ref<string | null>(null);

const activeSession = computed(() => sessionStore.activeSession);
const modeOptions = computed(() => {
  const session = activeSession.value as ({ modes?: SessionMode[] } | null);
  return session?.modes ?? [];
});
const hasModeChoices = computed(() => modeOptions.value.length > 1);
const isPending = computed(() => pendingModeId.value !== null);
const selectedModeId = computed(() => pendingModeId.value ?? activeSession.value?.currentModeId ?? '');
const selectedMode = computed(() =>
  modeOptions.value.find((mode) => mode.id === selectedModeId.value)
);
const selectedModeLabel = computed(() => (selectedMode.value?.name ?? selectedModeId.value) || 'Session mode');
const selectedModeIcon = computed(() => getOptionIcon(selectedModeId.value, 'mode'));

async function handleModeChange(value: AcceptableValue): Promise<void> {
  if (typeof value !== 'string' || value === activeSession.value?.currentModeId || isPending.value) return;
  pendingModeId.value = value;
  errorMessage.value = null;
  try {
    await sessionStore.setSessionMode(value);
  } catch {
    errorMessage.value = 'Failed to change session mode.';
  } finally {
    if (pendingModeId.value === value) pendingModeId.value = null;
  }
}
</script>

<template>
  <div v-if="hasModeChoices" class="flex flex-col gap-1" data-test="session-mode-switcher">
    <Select :model-value="selectedModeId" :disabled="isPending" @update:model-value="handleModeChange">
      <SelectTrigger class="h-8 w-[180px] gap-2 text-xs" aria-label="Session mode">
        <span class="shrink-0 text-body-sm">{{ selectedModeIcon }}</span>
        <span class="min-w-0 flex-1 truncate text-left font-medium">{{ selectedModeLabel }}</span>
        <span v-if="isPending" class="text-xs text-text-secondary" aria-live="polite">…</span>
      </SelectTrigger>
      <SelectContent>
        <SelectItem
          v-for="mode in modeOptions"
          :key="mode.id"
          :value="mode.id"
          class="text-xs"
        >
          <span class="mr-2">{{ getOptionIcon(mode.id, 'mode') }}</span>
          <span>{{ mode.name }}</span>
          <span v-if="mode.description" class="block text-xs text-text-secondary">
            {{ mode.description }}
          </span>
        </SelectItem>
      </SelectContent>
    </Select>
    <span v-if="errorMessage" role="status" class="text-xs text-danger">{{ errorMessage }}</span>
  </div>
</template>
