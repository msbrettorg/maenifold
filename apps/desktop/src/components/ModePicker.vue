<script setup lang="ts">
import { Button } from '@/components/ui/button';
import type { SessionMode } from '../lib/types';

defineProps<{
  modes: SessionMode[];
  currentModeId: string | null;
}>();

const emit = defineEmits<{
  'select-mode': [{ modeId: string }];
}>();

function selectMode(modeId: string): void {
  emit('select-mode', { modeId });
}
</script>

<template>
  <div v-if="modes.length > 0" class="flex flex-wrap items-center gap-2 px-4 pt-3" data-testid="mode-picker">
    <Button
      v-for="mode in modes"
      :key="mode.id"
      variant="outline"
      size="xs"
      class="rounded-full"
      :class="mode.id === currentModeId ? 'bg-accent text-accent-foreground hover:bg-accent hover:text-accent-foreground' : 'text-text-secondary'"
      :aria-pressed="mode.id === currentModeId"
      :data-testid="`mode-picker-pill-${mode.id}`"
      @click="selectMode(mode.id)"
    >
      {{ mode.name }}
    </Button>
  </div>
</template>
