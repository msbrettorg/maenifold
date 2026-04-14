<script setup lang="ts">
import { Button } from '@/components/ui/button';
const props = defineProps<{
  designMode: boolean;
  hasSelection: boolean;
  paletteOpen: boolean;
  showPaletteToggle: boolean;
}>();

const emit = defineEmits<{
  (e: 'autoLayout', direction: 'LR' | 'TB'): void;
  (e: 'deleteSelected'): void;
  (e: 'fitView'): void;
  (e: 'togglePalette'): void;
}>();
</script>

<template>
  <div class="flow-toolbar" role="toolbar" aria-label="Pipeline actions">
    <Button type="button" variant="ghost" @click="emit('autoLayout', 'LR')">↔ Layout LR</Button>
    <Button type="button" variant="ghost" @click="emit('autoLayout', 'TB')">↕ Layout TB</Button>
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

<style scoped>
.flow-toolbar { position: absolute; top: 0.75rem; right: 0.75rem; z-index: 5; display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 0.5rem; }
</style>
