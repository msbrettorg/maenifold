<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Command, CommandItem, CommandList } from '@/components/ui/command';
import type { AvailableCommand } from '../lib/types';

const props = defineProps<{
  commands: AvailableCommand[];
  filter: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  select: [command: AvailableCommand];
  close: [];
}>();

const commandListId = 'command-palette-list';
const selectedCommandName = ref('');

// Filter commands based on input
const filteredCommands = computed(() => {
  const filterText = props.filter.toLowerCase();
  if (!filterText) return props.commands;
  return props.commands.filter(cmd => 
    cmd.name.toLowerCase().startsWith(filterText) ||
    (cmd.description ?? '').toLowerCase().includes(filterText)
  );
});

const selectedCommand = computed(() =>
  filteredCommands.value.find((cmd) => cmd.name === selectedCommandName.value) ?? filteredCommands.value[0]
);

const commandFilterValue = computed(() => (props.filter ? `/${props.filter}` : '/'));

function commandOptionId(commandName: string): string {
  return `command-palette-option-${commandName.replace(/[^a-z0-9]+/gi, '-').toLowerCase()}`;
}

function syncSelection() {
  const firstCommand = filteredCommands.value[0];
  if (!firstCommand) {
    selectedCommandName.value = '';
    return;
  }

  if (!filteredCommands.value.some((cmd) => cmd.name === selectedCommandName.value)) {
    selectedCommandName.value = firstCommand.name;
  }
}

// Reset selection when filter or visibility changes
watch([() => props.filter, () => props.visible, filteredCommands], syncSelection, { immediate: true });

function moveSelection(offset: number) {
  if (filteredCommands.value.length === 0) return;

  const currentIndex = filteredCommands.value.findIndex((cmd) => cmd.name === selectedCommandName.value);
  const nextIndex = currentIndex === -1
    ? 0
    : (currentIndex + offset + filteredCommands.value.length) % filteredCommands.value.length;

  selectedCommandName.value = filteredCommands.value[nextIndex]?.name ?? '';
}

function handleKeyDown(event: KeyboardEvent) {
  if (!props.visible || filteredCommands.value.length === 0) return;

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      moveSelection(1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      moveSelection(-1);
      break;
    case 'Enter':
      event.preventDefault();
      if (selectedCommand.value) {
        emit('select', selectedCommand.value);
      }
      break;
    case 'Escape':
      event.preventDefault();
      emit('close');
      break;
    case 'Tab':
      event.preventDefault();
      if (selectedCommand.value) {
        emit('select', selectedCommand.value);
      }
      break;
  }
}

// Expose keyboard handler for parent to call
defineExpose({ handleKeyDown });

function selectCommand(cmd: AvailableCommand) {
  selectedCommandName.value = cmd.name;
  emit('select', cmd);
}

function handleSelectionChange(value: unknown) {
  if (typeof value === 'string') {
    selectedCommandName.value = value;
  }
}
</script>

<template>
  <div v-if="visible" class="command-palette">
    <Command
      :model-value="selectedCommandName"
      class="command-root"
      @update:model-value="handleSelectionChange"
    >
      <div class="command-input-row">
        <input
          class="command-input"
          type="text"
          role="combobox"
          aria-label="Command palette"
                    aria-autocomplete="list"
          :aria-controls="commandListId"
          :aria-expanded="visible"
          :value="commandFilterValue"
          readonly
          tabindex="-1"
        >
      </div>

      <CommandList :id="commandListId" class="command-list">
        <div v-if="filteredCommands.length === 0" class="command-empty">
          No matching commands.
        </div>
        <CommandItem
          v-for="cmd in filteredCommands"
          :id="commandOptionId(cmd.name)"
          :key="cmd.name"
          :value="cmd.name"
          class="command-item"
          @mouseenter="selectedCommandName = cmd.name"
          @select="selectCommand(cmd)"
        >
          <div class="command-row">
            <span class="command-name">/{{ cmd.name }}</span>
            <span class="command-description">{{ cmd.description }}</span>
            <span v-if="cmd.input?.hint" class="command-hint">{{ cmd.input.hint }}</span>
          </div>
        </CommandItem>
      </CommandList>
    </Command>

    <div v-if="selectedCommand?.description" class="command-tooltip">
      {{ selectedCommand.description }}
    </div>
  </div>
</template>

<style scoped>
/* Light theme VS Code style */
.command-palette {
  position: absolute;
  bottom: 100%;
  left: 0;
  width: 400px;
  margin-bottom: 6px;
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 4px 24px color-mix(in srgb, var(--background) 10%, transparent);
  max-height: 320px;
  overflow: visible;
  z-index: 100;
}

.command-root {
  padding: 0;
  border-radius: 8px;
  background: var(--background);
}

.command-input-row {
  display: flex;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border);
}

.command-input {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--foreground);
  font: inherit;
  font-size: var(--text-body-sm);
}

.command-input:focus {
  outline: none;
}

.command-list {
  max-height: 320px;
  overflow-y: auto;
}

.command-item {
  border-radius: 0;
  border-bottom: 1px solid var(--border);
  background: var(--background);
}

.command-item:last-child {
  border-bottom: none;
}

.command-empty {
  padding: 1rem;
  color: var(--muted-foreground);
  font-size: var(--text-body-sm);
  text-align: center;
}

.command-tooltip {
  position: absolute;
  left: calc(100% + 8px);
  top: 0;
  width: 220px;
  background: var(--card);
  color: var(--foreground);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 4px 16px color-mix(in srgb, var(--background) 12%, transparent);
  padding: 8px 12px;
  font-size: var(--text-body-sm);
  white-space: normal;
  z-index: 200;
  word-break: break-word;
}

/* Command row: command and description on same line */
.command-row {
  display: flex;
  align-items: center;
  width: 100%;
  gap: 0;
}
.command-name {
  font-weight: var(--weight-title);
  color: var(--ring);
  font-family: var(--font-code);
  font-size: var(--text-body-sm);
  min-width: 110px;
  max-width: 140px;
  flex-shrink: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.command-description {
  color: var(--muted-foreground);
  font-size: var(--text-body-sm);
  margin-left: 18px;
  flex: 1 1 auto;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
