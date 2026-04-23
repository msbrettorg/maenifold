<script setup lang="ts">
import { Button } from '@/components/ui/button';
const props = defineProps<{
  designMode: boolean;
  hasSelection: boolean;
  layoutDirection: 'LR' | 'TB';
  paletteOpen: boolean;
  showPaletteToggle: boolean;
}>();

const emit = defineEmits<{
  (e: 'autoLayout', direction: 'LR' | 'TB'): void;
  (e: 'setLayoutDirection', direction: 'LR' | 'TB'): void;
  (e: 'deleteSelected'): void;
  (e: 'fitView'): void;
  (e: 'togglePalette'): void;
}>();

function requestAutoLayout(direction: 'LR' | 'TB'): void {
  emit('setLayoutDirection', direction);
  emit('autoLayout', direction);
}
</script>

<template>
  <div class="absolute top-3 right-3 z-[5] flex flex-wrap justify-end gap-2" role="toolbar" aria-label="Pipeline actions">
    <Button
      type="button"
      :variant="props.layoutDirection === 'LR' ? 'secondary' : 'ghost'"
      @click="requestAutoLayout('LR')"
    >
      ↔ Layout LR
    </Button>
    <Button
      type="button"
      :variant="props.layoutDirection === 'TB' ? 'secondary' : 'ghost'"
      @click="requestAutoLayout('TB')"
    >
      ↕ Layout TB
    </Button>
    <Button v-if="designMode && hasSelection" type="button" variant="destructive" @click="emit('deleteSelected')">
      Delete Selected
    </Button>
    <Button v-if="designMode" type="button" variant="ghost" @click="emit('fitView')">⊞ Fit View</Button>
    <Button
      v-if="showPaletteToggle"
      type="button"
      variant="ghost"
      :aria-pressed="props.paletteOpen"
      @click="emit('togglePalette')"
    >
      {{ props.paletteOpen ? '✕ Palette' : '+ Palette' }}
    </Button>
    <slot />
  </div>
</template>
