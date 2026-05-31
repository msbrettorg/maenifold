<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { SlashCommand } from '../lib/types';

const props = defineProps<{
  commands: SlashCommand[];
  filter: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  select: [command: SlashCommand];
  close: [];
}>();

const selectedIndex = ref(0);

// Filter commands based on input
const filteredCommands = computed(() => {
  const filterText = props.filter.toLowerCase();
  if (!filterText) return props.commands;
  return props.commands.filter(cmd => 
    cmd.name.toLowerCase().startsWith(filterText) ||
    cmd.description.toLowerCase().includes(filterText)
  );
});

// Reset selection when filter or visibility changes
watch([() => props.filter, () => props.visible], () => {
  selectedIndex.value = 0;
});

function handleKeyDown(event: KeyboardEvent) {
  if (!props.visible || filteredCommands.value.length === 0) return;
  
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % filteredCommands.value.length;
      break;
    case 'ArrowUp':
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value - 1 + filteredCommands.value.length) % filteredCommands.value.length;
      break;
    case 'Enter':
      event.preventDefault();
      if (filteredCommands.value[selectedIndex.value]) {
        emit('select', filteredCommands.value[selectedIndex.value]);
      }
      break;
    case 'Escape':
      event.preventDefault();
      emit('close');
      break;
    case 'Tab':
      event.preventDefault();
      if (filteredCommands.value[selectedIndex.value]) {
        emit('select', filteredCommands.value[selectedIndex.value]);
      }
      break;
  }
}

// Expose keyboard handler for parent to call
defineExpose({ handleKeyDown });

function selectCommand(cmd: SlashCommand) {
  emit('select', cmd);
}
</script>

<template>
  <div 
    v-if="visible && filteredCommands.length > 0"
    ref="paletteRef"
    class="command-palette"
  >
    <div class="command-list">
      <div 
        v-for="(cmd, index) in filteredCommands"
        :key="cmd.name"
        :class="['command-item', { selected: index === selectedIndex }]"
        @click="selectCommand(cmd)"
        @mouseenter="selectedIndex = index"
      >
        <div class="command-row">
          <span class="command-name">/{{ cmd.name }}</span>
          <span class="command-description">{{ cmd.description }}</span>
        </div>
      </div>
    </div>
    <!-- Tooltip on right side for selected item -->
    <div 
      v-if="filteredCommands[selectedIndex]?.description"
      class="command-tooltip"
    >
      {{ filteredCommands[selectedIndex].description }}
    </div>
  </div>
</template>

<style scoped>
/* Theme-aware palette that respects the app's CSS variables (light/dark)
   rather than the hardcoded #fff / #2563eb that used to clash with the
   dark sidebar + chat area. */
.command-palette {
  position: absolute;
  bottom: 100%;
  left: 0;
  width: 400px;
  margin-bottom: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.18);
  max-height: 320px;
  overflow: visible;
  z-index: 100;
}

@media (prefers-color-scheme: dark) {
  .command-palette {
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.55);
  }
}

.command-list {
  max-height: 320px;
  overflow-y: auto;
}

.command-item {
  padding: 8px 16px;
  cursor: pointer;
  border-bottom: 1px solid var(--border-color);
  background: transparent;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  transition: background 0.12s, color 0.12s;
  position: relative;
}
.command-item:last-child {
  border-bottom: none;
}
.command-item:hover:not(.selected) {
  background: var(--bg-hover);
}
.command-item.selected {
  background: var(--bg-primary);
  color: #fff;
}
.command-item.selected .command-name,
.command-item.selected .command-description {
  color: #fff;
}

.command-tooltip {
  position: absolute;
  left: calc(100% + 8px);
  top: 0;
  width: 220px;
  background: var(--bg-main);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
  padding: 8px 12px;
  font-size: 0.95em;
  white-space: normal;
  z-index: 200;
  word-break: break-word;
}

@media (prefers-color-scheme: dark) {
  .command-tooltip {
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.55);
  }
}

/* Command row: command and description on same line */
.command-row {
  display: flex;
  align-items: center;
  width: 100%;
  gap: 0;
}
.command-name {
  font-weight: 600;
  color: var(--text-accent);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 1em;
  min-width: 110px;
  max-width: 140px;
  flex-shrink: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.command-description {
  color: var(--text-secondary);
  font-size: 0.97em;
  margin-left: 18px;
  flex: 1 1 auto;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
