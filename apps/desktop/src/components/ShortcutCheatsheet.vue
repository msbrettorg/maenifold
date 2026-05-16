<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  visible: boolean;
}>();

const modifier = computed(() => (navigator.platform.includes('Mac') ? 'Cmd' : 'Ctrl'));
</script>

<template>
  <section
    v-if="props.visible"
    class="rounded-[8px] border border-border bg-card p-4 text-sm text-foreground shadow-sm"
    aria-label="Keyboard shortcuts"
  >
    <h2 class="mb-2 font-semibold">Keyboard shortcuts</h2>
    <ul class="space-y-1">
      <li>{{ modifier }}+K — Open command palette</li>
      <li>Shift+? — Open shortcut cheatsheet</li>
    </ul>
  </section>
</template>
