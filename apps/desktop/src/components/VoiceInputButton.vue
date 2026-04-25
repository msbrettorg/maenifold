<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { Mic } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useVoiceHandyStore } from '@/stores/voice-handy';

const voiceHandyStore = useVoiceHandyStore();
const { installed, running, installHint, enabled, lastError } = storeToRefs(voiceHandyStore);

const isElectron = computed(() =>
  typeof window !== 'undefined'
  && !!window.electronAPI
  && typeof window.electronAPI.voiceCheck === 'function'
);

onMounted(() => {
  if (isElectron.value && installed.value === null) {
    void voiceHandyStore.check();
  }
});

const tooltipText = computed(() => {
  if (lastError.value) {
    return `Voice error: ${lastError.value}`;
  }

  if (installed.value === false) {
    return `Install Handy: ${installHint.value}`;
  }

  if (running.value) {
    return 'Open Handy — use its hotkey to record';
  }

  return 'Launch Handy to dictate';
});
</script>

<template>
  <TooltipProvider v-if="isElectron && enabled" :delay-duration="150">
    <Tooltip>
      <TooltipTrigger as-child>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="relative h-8 w-8 rounded-[4px]"
          :class="{ 'opacity-40': installed === false }"
          :aria-label="tooltipText"
          @click="installed !== false ? voiceHandyStore.launch() : undefined"
        >
          <Mic class="h-4 w-4" />
          <span
            v-if="lastError || running"
            aria-hidden="true"
            class="absolute bottom-0 right-0 h-1.5 w-1.5 rounded-full"
            :class="lastError ? 'bg-danger' : 'bg-success'"
          />
        </Button>
      </TooltipTrigger>
      <TooltipContent>
        <p>{{ tooltipText }}</p>
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>
