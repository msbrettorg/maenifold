<script setup lang="ts">
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
    <button type="button" class="btn btn-ghost" @click="emit('autoLayout', 'LR')">↔ Layout LR</button>
    <button type="button" class="btn btn-ghost" @click="emit('autoLayout', 'TB')">↕ Layout TB</button>
    <button v-if="designMode && hasSelection" type="button" class="btn btn-outline-danger" @click="emit('deleteSelected')">
      Delete Selected
    </button>
    <button v-if="designMode" type="button" class="btn btn-ghost" @click="emit('fitView')">⊞ Fit View</button>
    <button
      v-if="showPaletteToggle"
      type="button"
      class="btn btn-ghost"
      :aria-pressed="props.paletteOpen"
      @click="emit('togglePalette')"
    >
      {{ props.paletteOpen ? '✕ Palette' : '+ Palette' }}
    </button>
    <slot />
  </div>
</template>

<style scoped>
.flow-toolbar { position: absolute; top: 0.75rem; right: 0.75rem; z-index: 5; display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 0.5rem; }
</style>
